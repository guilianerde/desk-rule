use base::message_proto::{
    message, video_frame, ControlKey, KeyEvent, LoginRequest, Message, MouseEvent, OptionMessage,
    PublicKey, SupportedDecoding,
};
use hbb_common::{
    config::{self, Config},
    password_security,
    protobuf::Message as _,
    sha2::{Digest, Sha256},
    sodiumoxide::crypto::sign,
    tcp::KxTranscript,
    tokio::net::{TcpListener, TcpStream},
    Stream,
};

async fn receive(peer: &mut Stream) -> Message {
    let packet = hbb_common::timeout(8000, peer.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    Message::parse_from_bytes(&packet).unwrap()
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    assert!(librustdesk::common::global_init());
    *config::APP_NAME.write().unwrap() = "desk-rule-windows-e2e".into();
    let dpi_result = unsafe { winapi::um::shellscalingapi::SetProcessDpiAwareness(2) };
    println!("Win32 DPI awareness result: {dpi_result:#x}");
    config::OVERWRITE_SETTINGS.write().unwrap().extend([
        (
            "verification-method".into(),
            "use-temporary-password".into(),
        ),
        ("approve-mode".into(), "password".into()),
        ("stop-service".into(), "N".into()),
        ("enable-keyboard".into(), "Y".into()),
    ]);
    let _cm_listener = librustdesk::ipc::new_listener("_cm").await.unwrap();
    let (_, pk) = Config::get_key_pair();
    let pin = sign::PublicKey::from_slice(&pk).unwrap();
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let address = listener.local_addr().unwrap();
    let host = tokio::spawn(async move {
        let host = librustdesk::new();
        let (stream, addr) = listener.accept().await.unwrap();
        let local = stream.local_addr().unwrap();
        librustdesk::create_tcp_connection(
            host,
            Stream::from(stream, local),
            addr,
            true,
            Default::default(),
        )
        .await
        .unwrap();
    });

    let socket = TcpStream::connect(address).await.unwrap();
    let local = socket.local_addr().unwrap();
    let mut peer = Stream::from(socket, local);
    let Some(message::Union::SignedId(hello)) = receive(&mut peer).await.union else {
        panic!("missing host identity")
    };
    let (_, host_key, _, advertised) =
        librustdesk::common::decode_id_pk_dtls(&hello.id, &pin).unwrap();
    let (initiator_pk, symmetric_value, key) =
        librustdesk::common::create_symmetric_key_msg(host_key);
    let mut packet = Message::new();
    packet.set_public_key(PublicKey {
        asymmetric_value: initiator_pk.clone(),
        symmetric_value,
        kx_version: advertised,
        ..Default::default()
    });
    peer.send(&packet).await.unwrap();
    peer.set_negotiated_key(
        key,
        true,
        &KxTranscript {
            initiator_pk: &initiator_pk,
            responder_pk: &host_key,
            advertised,
            picked: advertised,
        },
    )
    .unwrap();
    let Some(message::Union::Hash(hash)) = receive(&mut peer).await.union else {
        panic!("missing password challenge")
    };
    let mut stage1 = Sha256::new();
    stage1.update(password_security::temporary_password().as_bytes());
    stage1.update(hash.salt.as_bytes());
    let mut stage2 = Sha256::new();
    stage2.update(stage1.finalize());
    stage2.update(hash.challenge.as_bytes());
    packet.set_login_request(LoginRequest {
        username: address.to_string(),
        my_id: "windows-e2e".into(),
        my_name: "Windows fixture".into(),
        version: librustdesk::VERSION.into(),
        session_id: 1,
        password: stage2.finalize().to_vec().into(),
        option: Some(OptionMessage {
            supported_decoding: Some(SupportedDecoding {
                ability_vp9: 1,
                ..Default::default()
            })
            .into(),
            ..Default::default()
        })
        .into(),
        ..Default::default()
    });
    peer.send(&packet).await.unwrap();
    let display = loop {
        if let Some(message::Union::LoginResponse(response)) = receive(&mut peer).await.union {
            assert!(
                response.has_peer_info(),
                "login refused: {}",
                response.error()
            );
            break response
                .peer_info()
                .displays
                .first()
                .expect("missing Windows display")
                .clone();
        }
    };
    assert!(display.width > 100 && display.height > 100);

    let mut decoder = scrap::codec::Decoder::new(scrap::CodecFormat::VP9, None);
    let mut rgb = scrap::ImageRgb::new(scrap::ImageFormat::ARGB, 1);
    loop {
        if let Some(message::Union::VideoFrame(frame)) = receive(&mut peer).await.union {
            if let Some(union @ video_frame::Union::Vp9s(_)) = frame.union {
                if decoder
                    .handle_video_frame(
                        &union,
                        &mut rgb,
                        &mut Default::default(),
                        &mut false,
                        &mut None,
                    )
                    .unwrap()
                {
                    assert_eq!(
                        (rgb.w, rgb.h),
                        (display.width as usize, display.height as usize)
                    );
                    assert!(!rgb.raw.is_empty());
                    println!(
                        "Decoded an encrypted remote Windows frame: {}x{}",
                        rgb.w, rgb.h
                    );
                    break;
                }
            }
        }
    }

    let origin = librustdesk::get_cursor_pos().expect("Windows input desktop is not accessible");
    let mut target = (display.x + 43, display.y + 51);
    // Windows normalizes SendInput coordinates to a 16-bit grid.
    let near = |p: (i32, i32)| p.0.abs_diff(target.0) <= 2 && p.1.abs_diff(target.1) <= 2;
    if near(origin) {
        target.0 += 20;
    }
    packet.set_mouse_event(MouseEvent {
        x: target.0,
        y: target.1,
        ..Default::default()
    });
    peer.send(&packet).await.unwrap();
    let pointer = hbb_common::timeout(3000, async {
        loop {
            if let Some(p) = librustdesk::get_cursor_pos() {
                if p.0.abs_diff(target.0) <= 2 && p.1.abs_diff(target.1) <= 2 {
                    break p;
                }
            }
            hbb_common::sleep(0.05).await;
        }
    })
    .await;
    println!(
        "Mouse origin: {origin:?}, requested: {target:?}, observed: {:?}",
        librustdesk::get_cursor_pos()
    );

    let f8_is_down =
        || unsafe { winapi::um::winuser::GetAsyncKeyState(winapi::um::winuser::VK_F8) < 0 };
    assert!(!f8_is_down());
    let mut keyboard = KeyEvent {
        down: true,
        ..Default::default()
    };
    keyboard.set_control_key(ControlKey::F8);
    packet.set_key_event(keyboard.clone());
    peer.send(&packet).await.unwrap();
    let pressed = hbb_common::timeout(3000, async {
        while !f8_is_down() {
            hbb_common::sleep(0.05).await;
        }
    })
    .await;
    keyboard.down = false;
    packet.set_key_event(keyboard);
    peer.send(&packet).await.unwrap();
    pressed.expect("remote key event did not reach the Windows input desktop");
    hbb_common::timeout(3000, async {
        while f8_is_down() {
            hbb_common::sleep(0.05).await;
        }
    })
    .await
    .expect("remote key was not released");
    println!(
        "Windows input desktop received the remote mouse event and the key press and release."
    );

    drop(peer);
    hbb_common::timeout(3000, host).await.unwrap().unwrap();
}

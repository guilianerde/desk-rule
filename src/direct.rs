use std::net::{IpAddr, SocketAddr};

use base::message_proto::{message, Message, PublicKey};
#[cfg(not(target_os = "ios"))]
use hbb_common::{
    allow_err, log,
    tokio::{self, time::Duration},
};
use hbb_common::{
    anyhow::anyhow,
    bail,
    config::{self, LocalConfig, CONNECT_TIMEOUT},
    protobuf::Message as _,
    sodiumoxide::crypto::sign,
    tcp::{KxTranscript, KX_VERSION_LATEST},
    timeout,
    tokio::net::TcpStream,
    ResultType, Stream,
};

pub const APP_NAME: &str = "desk-rule";
pub const PORT: u16 = 21118;

pub fn init_config() {
    use base::config::keys::*;
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| {
        *config::APP_NAME.write().unwrap() = APP_NAME.to_owned();
        config::DEFAULT_SETTINGS
            .write()
            .unwrap()
            .insert("stop-service".into(), "Y".into());
        let mut options = config::OVERWRITE_SETTINGS.write().unwrap();
        for key in [
            OPTION_ENABLE_AUDIO,
            OPTION_ENABLE_CAMERA,
            OPTION_ENABLE_CLIPBOARD,
            OPTION_ENABLE_FILE_TRANSFER,
            OPTION_ENABLE_REMOTE_PRINTER,
            OPTION_ENABLE_REMOTE_RESTART,
            OPTION_ENABLE_RECORD_SESSION,
            OPTION_ENABLE_BLOCK_INPUT,
            OPTION_ENABLE_PRIVACY_MODE,
            OPTION_ENABLE_TERMINAL,
            OPTION_ENABLE_TUNNEL,
            OPTION_ENABLE_TRUSTED_DEVICES,
            OPTION_ENABLE_PERM_CHANGE_IN_ACCEPT_WINDOW,
        ] {
            options.insert(key.into(), "N".into());
        }
        options.insert("access-mode".into(), "custom".into());
        options.insert(
            "verification-method".into(),
            "use-both-passwords".into(),
        );
        options.insert("approve-mode".into(), "password".into());
        options.insert("2fa".into(), String::new());
    });
}

pub fn endpoint(peer: &str) -> ResultType<SocketAddr> {
    let address = peer
        .parse::<IpAddr>()
        .map(|ip| SocketAddr::new(ip, PORT))
        .or_else(|_| peer.parse::<SocketAddr>());
    match address {
        Ok(address)
            if address.port() != 0
                && !address.ip().is_unspecified()
                && !address.ip().is_multicast() =>
        {
            Ok(address)
        }
        _ => bail!("Enter a device IP address"),
    }
}

pub async fn connect(peer: &str) -> ResultType<(Stream, Vec<u8>)> {
    let address = endpoint(peer)?;
    let fingerprint = LocalConfig::get_option_from_file(&format!("peer-fingerprint-{peer}"));
    let pk = hex::decode(fingerprint.split_whitespace().collect::<String>())
        .ok()
        .and_then(|pk| sign::PublicKey::from_slice(&pk))
        .ok_or_else(|| anyhow!("Enter the controlled device's fingerprint"))?;
    let stream = timeout(CONNECT_TIMEOUT, TcpStream::connect(address)).await??;
    let local = stream.local_addr()?;
    stream.set_nodelay(true)?;
    let mut stream = Stream::from(stream, local);
    authenticate_host(&mut stream, &pk).await?;
    Ok((stream, pk.0.to_vec()))
}

async fn authenticate_host(stream: &mut Stream, pk: &sign::PublicKey) -> ResultType<()> {
    stream.set_max_packet_length(4096);
    let bytes = timeout(CONNECT_TIMEOUT, stream.next())
        .await?
        .ok_or_else(|| anyhow!("The controlled device closed the connection"))??;
    let Some(message::Union::SignedId(hello)) = Message::parse_from_bytes(&bytes)?.union else {
        bail!("The controlled device did not start an encrypted Desk Rule session");
    };
    let (_, host_key, _, advertised) = crate::common::decode_id_pk_dtls(&hello.id, pk)
        .map_err(|_| anyhow!("The controlled device's fingerprint does not match"))?;
    if advertised != KX_VERSION_LATEST {
        bail!("The controlled device uses an unsupported encryption version");
    }
    let (initiator_pk, symmetric_value, key) = crate::common::create_symmetric_key_msg(host_key);
    let mut reply = Message::new();
    reply.set_public_key(PublicKey {
        asymmetric_value: initiator_pk.clone(),
        symmetric_value,
        kx_version: advertised,
        ..Default::default()
    });
    timeout(CONNECT_TIMEOUT, stream.send(&reply)).await??;
    stream.set_negotiated_key(
        key,
        true,
        &KxTranscript {
            initiator_pk: &initiator_pk,
            responder_pk: &host_key,
            advertised,
            picked: advertised,
        },
    )?;
    stream.set_max_packet_length(usize::MAX);
    Ok(())
}

#[cfg(not(target_os = "ios"))]
pub fn is_control_message(message: &Message) -> bool {
    use base::message_proto::{message::Union, misc};
    match message.union.as_ref() {
        Some(Union::LoginRequest(login)) => login.union.is_none(),
        Some(
            Union::TestDelay(_)
            | Union::MouseEvent(_)
            | Union::KeyEvent(_)
            | Union::PointerDeviceEvent(_),
        ) => true,
        Some(Union::Misc(m)) => matches!(
            m.union.as_ref(),
            Some(
                misc::Union::CloseReason(_)
                    | misc::Union::SwitchDisplay(_)
                    | misc::Union::CaptureDisplays(_)
                    | misc::Union::Option(_)
                    | misc::Union::RefreshVideo(_)
                    | misc::Union::RefreshVideoDisplay(_)
                    | misc::Union::VideoReceived(_)
                    | misc::Union::AutoAdjustFps(_)
                    | misc::Union::MessageQuery(_)
                    | misc::Union::RequestCursorData(_)
                    | misc::Union::SelectedSid(_)
            )
        ),
        _ => false,
    }
}

pub fn is_control_response(message: &Message) -> bool {
    use base::message_proto::{message::Union, misc};
    match message.union.as_ref() {
        Some(
            Union::Hash(_)
            | Union::LoginResponse(_)
            | Union::VideoFrame(_)
            | Union::CursorData(_)
            | Union::CursorId(_)
            | Union::CursorPosition(_)
            | Union::TestDelay(_)
            | Union::PeerInfo(_)
            | Union::MessageBox(_),
        ) => true,
        Some(Union::Misc(m)) => matches!(
            m.union.as_ref(),
            Some(
                misc::Union::PermissionInfo(_)
                    | misc::Union::SwitchDisplay(_)
                    | misc::Union::CloseReason(_)
                    | misc::Union::StopService(_)
                    | misc::Union::SupportedEncoding(_)
                    | misc::Union::FollowCurrentDisplay(_)
            )
        ),
        _ => false,
    }
}

#[cfg(not(target_os = "ios"))]
pub fn control_options(
    option: &base::message_proto::OptionMessage,
) -> base::message_proto::OptionMessage {
    base::message_proto::OptionMessage {
        image_quality: option.image_quality,
        custom_image_quality: option.custom_image_quality,
        custom_fps: option.custom_fps,
        supported_decoding: option.supported_decoding.clone(),
        show_remote_cursor: option.show_remote_cursor,
        follow_remote_cursor: option.follow_remote_cursor,
        follow_remote_window: option.follow_remote_window,
        disable_keyboard: option.disable_keyboard,
        ..Default::default()
    }
}

#[cfg(not(target_os = "ios"))]
pub async fn serve() {
    use crate::server::{self, ConnectionMeta};
    use config::Config;

    server::check_zombie();
    scrap::codec::test_av1();
    let server = server::new();
    let mut listener = None;
    loop {
        if Config::get_bool_option("stop-service") || crate::platform::installing_service() {
            if listener.take().is_some() {
                server.write().unwrap().close_connections();
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
            continue;
        }
        if listener.is_none() {
            match hbb_common::tcp::listen_any(PORT).await {
                Ok(bound) => {
                    log::info!("Desk Rule listening on TCP port {PORT}");
                    listener = Some(bound);
                }
                Err(err) => {
                    hbb_common::throttled_log!(
                        Duration::from_secs(60),
                        warn,
                        "Cannot listen on TCP port {PORT}: {err}"
                    );
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
            }
        }
        let Some(listener) = listener.as_ref() else {
            continue;
        };
        let Ok(Ok((stream, peer))) = timeout(1000, listener.accept()).await else {
            continue;
        };
        let Ok(local) = stream.local_addr() else {
            continue;
        };
        if Config::get_bool_option("stop-service") {
            continue;
        }
        let server = server.clone();
        tokio::spawn(async move {
            allow_err!(stream.set_nodelay(true));
            allow_err!(
                server::create_tcp_connection(
                    server,
                    Stream::from(stream, local),
                    peer,
                    true,
                    ConnectionMeta::default(),
                )
                .await
            );
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(target_os = "ios"))]
    #[test]
    fn direct_sessions_allow_input_and_video_but_reject_auxiliary_messages() {
        use base::message_proto::{
            ChatMessage, FileTransfer, KeyEvent, LoginRequest, Misc, VideoFrame,
        };

        let mut packet = Message::new();
        packet.set_login_request(LoginRequest::new());
        assert!(is_control_message(&packet));
        packet
            .mut_login_request()
            .set_file_transfer(FileTransfer::new());
        assert!(!is_control_message(&packet));

        packet.set_key_event(KeyEvent::new());
        assert!(is_control_message(&packet));
        packet.set_video_frame(VideoFrame::new());
        assert!(is_control_response(&packet));

        let mut misc = Misc::new();
        misc.set_chat_message(ChatMessage::new());
        packet.set_misc(misc);
        assert!(!is_control_message(&packet));
        assert!(!is_control_response(&packet));
    }

    #[test]
    fn accepts_ipv4_and_ipv6_endpoints() {
        for (input, expected) in [
            ("100.64.0.1", "100.64.0.1:21118"),
            ("192.168.1.2:3000", "192.168.1.2:3000"),
            ("fd7a:115c:a1e0::2", "[fd7a:115c:a1e0::2]:21118"),
            ("[fd7a:115c:a1e0::2]:21118", "[fd7a:115c:a1e0::2]:21118"),
        ] {
            assert_eq!(endpoint(input).unwrap().to_string(), expected);
        }
    }

    #[test]
    fn refuses_ids_names_and_invalid_destinations() {
        for input in [
            "123456789",
            "workstation:21118",
            "0.0.0.0",
            "224.0.0.1",
            "100.64.0.1:0",
            "100.64.0.1/r",
        ] {
            assert!(endpoint(input).is_err(), "{input}");
        }
    }

    #[cfg(not(target_os = "ios"))]
    #[hbb_common::tokio::test]
    async fn pinned_identity_enables_an_encrypted_session_in_both_directions() {
        use base::message_proto::{Hash, KeyEvent};
        use hbb_common::tokio::net::TcpListener;

        let (public, secret) = sign::gen_keypair();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let host = async {
            let (stream, _) = listener.accept().await.unwrap();
            let mut stream = Stream::from(stream, address);
            crate::server::send_host_identity(&mut stream, "host".into(), &secret)
                .await
                .unwrap();
            let mut hash = Message::new();
            hash.set_hash(Hash {
                challenge: "session challenge".into(),
                ..Default::default()
            });
            stream.send(&hash).await.unwrap();
            let bytes = stream.next().await.unwrap().unwrap();
            assert!(Message::parse_from_bytes(&bytes).unwrap().has_key_event());
        };
        let controller = async {
            let stream = TcpStream::connect(address).await.unwrap();
            let local = stream.local_addr().unwrap();
            let mut stream = Stream::from(stream, local);
            authenticate_host(&mut stream, &public).await.unwrap();
            let bytes = stream.next().await.unwrap().unwrap();
            let response = Message::parse_from_bytes(&bytes).unwrap();
            assert_eq!(response.hash().challenge, "session challenge");
            let mut input = Message::new();
            input.set_key_event(KeyEvent::new());
            stream.send(&input).await.unwrap();
        };
        timeout(5000, async { tokio::join!(host, controller) })
            .await
            .unwrap();
    }

    #[cfg(not(target_os = "ios"))]
    #[hbb_common::tokio::test]
    async fn refuses_a_host_whose_key_does_not_match_the_pinned_fingerprint() {
        use hbb_common::tokio::net::TcpListener;

        let (pinned, _) = sign::gen_keypair();
        let (_, unexpected) = sign::gen_keypair();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let host = async {
            let (stream, _) = listener.accept().await.unwrap();
            let mut stream = Stream::from(stream, address);
            assert!(
                crate::server::send_host_identity(&mut stream, "host".into(), &unexpected)
                    .await
                    .is_err()
            );
        };
        let controller = async {
            let stream = TcpStream::connect(address).await.unwrap();
            let local = stream.local_addr().unwrap();
            let mut stream = Stream::from(stream, local);
            let error = authenticate_host(&mut stream, &pinned).await.unwrap_err();
            assert!(error.to_string().contains("fingerprint does not match"));
        };
        timeout(5000, async { tokio::join!(host, controller) })
            .await
            .unwrap();
    }
}

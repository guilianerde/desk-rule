# desk-rule

Desk Rule 是基于 RustDesk 的点对点远程桌面。主控端直接连接被控端的 IPv4 或 IPv6 地址，默认使用 TCP 21118。只要两台设备之间已有 IP 路由，就可以使用局域网、Tailscale 或其他 VPN。

应用保留远程画面、鼠标、键盘、设备身份校验和临时密码。连接路径移除了 RustDesk 的 ID 注册、打洞、中继和在线查询。桌面会话只允许控制所需的协议消息。

## 使用

1. 两台设备都打开 Desk Rule。macOS 被控端需要在系统设置中允许 Desk Rule 使用「屏幕录制」和「辅助功能」。
2. 在被控端点 **Start sharing**，把可达 IP、界面显示的 **Fingerprint** 和 **Temporary password** 提供给主控端。
3. 在主控端输入被控端 IP 和指纹，点 **Connect**，按提示输入临时密码。裸 IPv4 或 IPv6 默认使用 21118，也可填写 `IPv4:端口` 或 `[IPv6]:端口`。
4. 在被控端点 **Stop sharing** 可关闭监听并结束已有会话。

共享默认关闭。首次连接前要从被控端核对指纹，Desk Rule 会在发送登录信息之前验证身份并加密 TCP 会话。

## 从源码构建

源码需要 `libs/hbb_common`。如果源码来自 Git，运行 `git submodule update --init --recursive`。本项目使用的上游 RustDesk 基线是 `fdffc37bb067d14389350937effa3269b52b656d`，对应的 hbb_common 是 `fae47e024dd83cef6aaecde0aa7d11731df11649`。

安装 Rust、C/C++ 工具链和原生依赖 `libvpx`、`libyuv`、`aom`、`opus`。使用 vcpkg 时设置 `VCPKG_ROOT`，使用对应架构的依赖。原生桌面构建可复用仓库中的 Sciter UI：

```sh
python3 res/inline-sciter.py
cargo build --locked --release --features inline --bin desk-rule
```

把平台对应的 [Sciter 运行库](https://github.com/c-smile/sciter-sdk) 放在二进制旁。macOS 应用包也可以把 `libsciter.dylib` 放在 `Contents/Frameworks`。Flutter 源码保留原有 Rust bridge 接口，Flutter 构建仍需要项目原有的生成绑定。

### Windows x64 便携包

在 Windows 上准备 Visual Studio 的 x64 C++ Build Tools、Rust MSVC 工具链、Python 3、LLVM 和已运行过 `bootstrap-vcpkg.bat` 的 vcpkg。vcpkg 可以使用 `vcpkg.json` 中记录的上游基线。然后在源码目录的 PowerShell 中运行：

```powershell
.\scripts\build-windows.ps1 -VcpkgRoot C:\vcpkg
```

生成的 `target/windows-portable/desk-rule-windows-x64.zip` 解压后可直接打开 `desk-rule.exe`。请让 `sciter.dll` 留在同一目录。如果 Windows 防火墙挡住了入站连接，请在 Windows 上允许这台主控设备访问 TCP 21118。GitHub 仓库也可手动运行 **Desk Rule for Windows** 工作流生成同一个便携包。

运行直连协议测试：

```sh
cargo test --locked --lib direct::tests
```

上游代码和本项目的许可见 [LICENCE](LICENCE)。

# FastGithub-GUI

把 [FastGithub](https://github.com/creazyboyone/FastGithub) 加速内核包装成**带托盘开关的桌面应用**：
点一下开启加速，再点一下关闭并还原所有设置。支持 macOS / Windows / Linux。

> 本项目是上游 FastGithub 的图形界面外壳，加速能力来自上游内核。许可证与署名见
> [`LICENSE`](LICENSE) 与 [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md)。

## 它做什么 / 不做什么

- **做**：在本机起一个 HTTP 代理（`127.0.0.1:38457`），把 GitHub 相关域名解析到可达的 IP，
  并用一张自签 CA 证书让浏览器与 `git` 信任该代理。开启时自动设置系统代理与 git 配置，
  关闭时全部还原。
- **不做**：它**不是 VPN**，也**不会提升你的带宽**。下载速度仍受你的网络出口限制；
  它主要改善的是 GitHub 的**可达性与访问延迟**。

## 安装

从 Releases（或网盘）下载对应平台的安装包：

| 平台 | 安装包 | 说明 |
| --- | --- | --- |
| macOS | `FastGithub GUI_*_aarch64.dmg` | Apple Silicon；内核为 x64，经 Rosetta 2 运行 |
| Windows | `FastGithub GUI_*_x64-setup.exe` | NSIS 安装器，按用户安装，**无需管理员** |
| Linux | `FastGithub GUI_*_amd64.AppImage` | 便携运行 |

### 绕过系统安全提示（免费开源工具未付费签名，属正常现象）

- **macOS**：首次打开若提示“无法验证开发者”，在“应用程序”里**右键 → 打开**，再点“打开”；
  或到“系统设置 → 隐私与安全性”点“仍要打开”。
- **Windows**：SmartScreen 提示时点“**更多信息 → 仍要运行**”。
- **Linux**：`chmod +x FastGithub*.AppImage` 后双击或命令行运行。

## 使用

左键点击托盘图标弹出菜单：

- **开启加速 / 关闭加速**：切换加速。开启中图标变色。
- **重新安装组件**：组件损坏时修复（macOS）。
- **卸载…**：还原代理与 git 配置、移除证书与后台组件（应用本体随后手动删除即可）。
- **退出**：关闭加速并退出。

开启后浏览器访问 github.com、`git clone/ls-remote/push` 都会走加速。关闭后一切恢复原样。

### 各平台行为

- **macOS**：首次开启需输入**一次管理员密码**安装后台守护组件；之后开关无需密码。
  CA 证书信任写入你的**登录钥匙串**（用户级，无需管理员）。
- **Windows**：全程**零管理员**。代理写当前用户注册表（HKCU），证书装入当前用户的
  Root 存储。若个别程序不认用户级证书，可反馈以便改用一次性 UAC 方案。
- **Linux**：GNOME 桌面自动设置系统代理（gsettings），证书装入 NSS 数据库（Firefox/Chrome）；
  可选通过 `pkexec` 安装系统 CA 以覆盖 `curl`（取消不视为失败）。**非 GNOME 桌面**需按提示
  手动设置代理。托盘依赖 `libayatana-appindicator3`，若托盘不显示请安装：
  `sudo apt install libayatana-appindicator3-1`。

### 崩溃保护

Windows / Linux 下有一个看门狗进程监视主程序：若主程序异常退出（崩溃/被强杀）而未正常关闭加速，
看门狗会**自动还原系统代理并结束内核**，避免残留代理导致断网。macOS 由后台守护进程负责同等还原。

## 常见问题

- **下载还是很慢？** 加速改善可达性与延迟，不增加带宽；下载吞吐取决于你的网络出口。
- **首次开启提示“内核未在 20s 内监听端口”？** 多为安全软件（如 CleanMyMac）拦截了内核执行。
  把应用目录加入安全软件白名单后，用菜单“重新安装组件”或重启应用再试。
- **DNS 解析慢？** 内核自带 dnscrypt-proxy 使用境外上游，首次解析可能偏慢，缓存后即快。
- **如何彻底卸载？** 菜单“卸载…”会清除证书/代理/后台组件；随后删除应用本体即可。

## 从源码构建

依赖：[.NET 8 SDK](https://dotnet.microsoft.com/download/dotnet/8.0)（≥ 8.0.10）、
[Rust](https://rustup.rs)（stable）、[Node.js](https://nodejs.org) 20+。

```bash
npm ci

# 1) 从官方源码编译内核（retarget 到 .NET 8；RID 可选 osx-x64 / win-x64 / linux-x64）
bash scripts/build-kernel.sh osx-x64

# 2) macOS 需先编译并暂存后台 helper（Windows/Linux 跳过此步）
cargo build --release --bin fastgithub-gui-helper --manifest-path src-tauri/Cargo.toml
mkdir -p src-tauri/resources/helper
cp src-tauri/target/release/fastgithub-gui-helper src-tauri/resources/helper/

# 3) 出包
npm run tauri build
```

为何从源码编译而非用官方预编译包：官方 v2.1.5 内置 .NET 7 运行时，在 macOS 15+ 上因
AppleCrypto 缺陷无法生成自签证书，且 .NET 7 已停止支持。故三平台统一 retarget 到 .NET 8（LTS）
自行编译。详见 [`scripts/build-kernel.sh`](scripts/build-kernel.sh)。

推送 `v*` 标签会触发 [GitHub Actions](.github/workflows/build.yml) 在三平台原生出包并附到 Release。

## 许可证

- 本项目：MIT（[`LICENSE`](LICENSE)）。
- 上游 FastGithub：MIT，`Copyright (c) 2021 老九`；dnscrypt-proxy：ISC。全文见
  [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md)，安装包内核目录亦随附原始 LICENSE。
- “GitHub” 及其标志是 GitHub, Inc. 的商标；本项目与其无关联。
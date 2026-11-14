# M1 macOS 验证记录

日期：2026-11-14 · 机器：macOS 26.5 (arm64) · 分支：feat/m1-macos

## 1. 内核路线修正（.NET 7 → .NET 8 源码编译）

**根因**：官方 FastGithub v2.1.5 预编译包内置 .NET 7 运行时。.NET 7 在 macOS 15 Sequoia+ 存在 AppleCrypto bug，`CertificateRequest.CreateSelfSigned` / `X509Certificate2.CopyWithPrivateKey` 抛 “The specified item is no longer valid”，导致内核启动即崩、无法生成自签 CA。该 bug 仅在 .NET 6.0.34/8.0.10/9.0 修复，.NET 7 已 EOL 永不修复。终端手动运行同样崩溃，排除会话/keychain/权限因素。

**修复**：三平台统一从官方源码（creazyboyone/FastGithub tag v2.1.5，commit 20ef8111，MIT）retarget 到 .NET 8 编译自包含单文件内核（`scripts/build-kernel.sh`，固定 tarball SHA256 + 校验 tag→commit）。retarget 最小改动：`Directory.Build.props` net7.0→net8.0；两个 `Microsoft.Extensions.Hosting.*` 7.0.0-rc*→8.0.0；编译设 `-p:InformationalVersion=2.1.5+source`（`ProductionVersion.Parse` 要求含 `+`）。产物未裁剪，osx-x64 约 49MB，经 Rosetta 运行。

**内核实测**（发布二进制 sha256 52f5999b…，从用户数据目录运行）：
- 启动日志 `FastGithub 启动完成，当前版本为 V2.1.5`。
- 生成 `cacert/fastgithub.cer` + `fastgithub.key`（.NET 8 证书生成正常）。
- TCP `127.0.0.1:38457` 监听成功。
- `curl --proxy http://127.0.0.1:38457 --cacert cacert/fastgithub.cer https://github.com/` → **HTTP 200**（MITM 代理 + 证书验证通过）。

## 2. 内核升级重释放（needs_extract）

旧版仅在 kernel 缺失时释放，升级后残留 .NET 7 内核。改为按 SHA256 比对资源内核与已释放内核，不一致即重释放（`kernel::needs_extract`，新增 sha2 依赖）。

## 3. 目录被安全软件标记导致 SIGKILL（已修复）

**现象**：同一内核二进制从 `~/Library/Application Support/FastGithubGUI/kernel/` 运行立即被 SIGKILL（rc=137，日志全空），从其它任意目录运行正常。

**定位**：
- 二进制哈希一致、xattr（com.apple.provenance）一致、无 quarantine/BSD flag/ACL，排除二进制与权限。
- 对照实验：`FastGithubGUI2`/`FGTest2`/`FGTest3`/`FastGithubGUIx`（新目录）均正常；仅原目录被杀。
- 把原目录改名后仍被杀 → 标记绑定 **目录 inode**，与路径名无关；新建同名目录立即正常。
- 机器装有 CleanMyMac 5（root Agent + HealthMonitor，配合系统 endpointsecurityd）。推断调试期 .NET 7 内核在该目录反复崩溃，被防护引擎标记，此后该目录内任何 exec 都被拒（SIGKILL，无崩溃报告/系统日志）。

**修复**：
- 环境：重建用户内核目录（新 inode）即恢复；已验证发布内核在新目录起代理并返回 HTTP 200。
- 代码加固：`kernel::extract_resources` 释放前 `remove_dir_all` 旧目录再重建，确保升级时获得新 inode，避免被标记的旧目录在升级后复发；同时清掉升级残留旧文件。
- 新装用户首次创建目录即为干净 inode，且 .NET 8 内核不再崩溃，不会触发此问题。

**给用户的提示**（README 常见问题，M4 补充）：若曾装过旧版且机器有 CleanMyMac 等防护软件，升级后仍报“内核未在 20s 内监听端口”，删除 `~/Library/Application Support/FastGithubGUI/kernel/` 后重新开启即可。

## 3b. 证书信任写入用户登录钥匙串（已修复）

**现象**：开启加速先后报「SecTrustSettingsSetTrustSettings: no user interaction was possible」（root daemon）与「SecCertificateAddToKeychain: Write permissions error」（GUI 以普通用户写系统钥匙串）。

**根因**：系统钥匙串（`/Library/Keychains/System.keychain`）的信任设置需要交互式管理员授权会话——root LaunchDaemon 无 GUI 会话被拒；GUI 以普通用户身份写系统钥匙串又被拒（security 不会自动提权）。

**修复**：改信任到**用户登录钥匙串**（user 域），普通用户即可写、无需管理员授权框：
- `cert::install_trusted`：`security add-trusted-cert -r trustRoot -k <login-keychain> <cer>`（去掉 `-d`，不写 admin 域）。`login_keychain()` 取 `security login-keychain` 路径。
- `cert::is_trusted(cer)`：`security verify-cert -c <cer> -p ssl` 返回码判定（真实 SSL 信任，而非仅证书存在）。
- `cert::remove_trusted(cn)`：`security delete-certificate -c <cn> -t <login-keychain>`。
- `backend::macos::enable` 端口就绪后 `is_trusted`→`install_trusted`；daemon `enable` 只设系统代理；`uninstall` 在 GUI 侧移除证书。

**验证**（真机 GUI 点击）：开启加速 rc=0 无授权框；`security find-certificate` 登录钥匙串含 FastGithub；`curl --proxy http://127.0.0.1:38457 https://github.com/`（**不带 --cacert**）→ HTTP 200，系统信任生效。

## 3c. 托盘双图标（已修复）

**根因**：`tauri.conf.json` 的 `app.trayIcon` 自动建一个无菜单托盘，`main.rs` 又用 `TrayIconBuilder` 手动建一个有菜单托盘 → 同进程两个图标。

**修复**：删除配置里的 `app.trayIcon`，仅保留手动 `TrayIconBuilder`（带菜单）。真机确认只剩一个图标，左键有菜单。

## 3d. 速度实测（代理开启 vs 直连）

| 项目 | 经 FastGithub | 直连 |
|---|---|---|
| 访问 github.com | ~15s | ~25s |
| 下载 ripgrep 2.5MB | ~38 KB/s | ~44 KB/s |
| git clone（warm） | 1.7s | — |
| git ls-remote | 11s | — |

FastGithub 防 DNS 污染 + 优选 IP + 改善访问延迟，**非带宽加速器**；下载吞吐受用户网络到 GitHub 线路限制。内核自带 dnscrypt 默认境外 DNS 上游从用户网络不可达（SYN_SENT），缓存过期后首次解析超时——本地化国内 DoH 可改善（用户决定暂不改）。

## 4. 真机 GUI 全流程清单

内核与代理路径已自动化验证通过；托盘操作由用户真机确认：

- [x] 单图标：删除配置 trayIcon 后仅一个托盘图标，左键有菜单。
- [x] 开启加速：内核监听 38457（IPv4+IPv6）；证书信任登录钥匙串（无授权框）；系统代理 = 127.0.0.1:38457；curl 经代理不带 --cacert → HTTP 200；git 两项配置写入。
- [x] 关闭加速：代理还原原值（127.0.0.1:7890 off）；git 两项配置移除；内核进程退出、端口释放；daemon `proxy_set_by_us=false`。
- [ ] 强杀应用：代理自动还原（daemon 长连接 EOF）。
- [ ] 重启 Mac：状态一致 Off，无残留代理。
- [ ] 卸载：证书信任、daemon、代理全部清除。

## 5. 交付物

- `.dmg`：`src-tauri/target/release/bundle/dmg/FastGithub GUI_0.1.0_aarch64.dmg`（arm64，内置 .NET 8 osx-x64 内核 + 原始 LICENSE）。
# FastGithub GUI 设计文档

- 日期：2026-11-14
- 状态：已批准（用户确认）
- 仓库/目录名：`fastgithub-gui`
- 应用名：FastGithub GUI

## 1. 背景与目标

用户网络访问 GitHub 不畅，已下载 FastGithub 命令行工具（creazyboyone/FastGithub 维护版，上游原仓库 dotnetcore/fastgithub 已删除），但不喜欢命令行。目标：把 FastGithub 包装成**可安装、带图形界面、一键开关**的桌面应用，覆盖 macOS / Windows / Linux 三平台，每平台交付一个安装包（用户放网盘分发），并整理完整源码供用户推送到 GitHub 开源。

FastGithub 在桌面平台的工作机制：本机 HTTP 代理 `127.0.0.1:38457` + 首次运行生成的自签 CA 证书（`cacert/fastgithub.cer`）+ 系统代理指向它。它不是 VPN，不做流量额外加密。

## 2. 范围

**目标**
- 三平台托盘应用：图标 + 原生菜单（状态、开关、重新安装组件、卸载、退出）。
- 一键开关全自动：开 = 启动内核 + 信任证书 + 设置系统代理 + 配置 git；关 = 全部还原。
- git 加速仅作用于 `https://github.com` 前缀，不关闭 SSL 验证，不影响其他仓库。
- 防断网：应用退出或崩溃时自动还原系统代理。
- 每平台一个安装包：macOS `.dmg`、Windows NSIS `.exe`、Linux `.AppImage`。
- GitHub Actions 三平台 CI 出包；完整源码 + 中文 README + 许可证整理。

**非目标（首版不做）**
- 翻墙/游戏加速（上游本身不支持）。
- 开机自启、开机自动加速（默认关闭，用户手动开）。
- Intel Mac（首版仅 arm64；现有 x64 内核可经 Rosetta 回退）。
- KDE 等非 GNOME 桌面的自动代理（给手动指引）。
- Firefox 证书自动导入（给手动指引）。
- 付费代码签名（README 说明安全提示绕过步骤）。

## 3. 总体架构

一套代码，Tauri 2（Rust + 极简静态前端，托盘应用无窗口）：

```
┌─────────────────────────────────────────────┐
│ Tauri 托盘应用（普通用户权限）               │
│  托盘图标/菜单 · 开关状态机 · git 配置       │
│  内核释放/管理 · 日志 · 错误提示             │
└──────────────┬──────────────────────────────┘
               │ 统一 PlatformBackend 接口
   ┌───────────┼───────────────┐
   ▼           ▼               ▼
 macOS       Windows         Linux
 root 守护   用户级代理/证书  用户级代理/证书
 进程        + 看门狗进程     + 看门狗进程
 (LaunchDaemon)              (可选 pkexec 系统 CA)
```

- **共享核心（Rust）**：状态机、git 配置、内核管理、日志、错误类型，三平台共用。
- **PlatformBackend trait**：`install_components / enable / disable / status / uninstall`，各平台一个实现。
- **macOS helper**：同一 Cargo 工程第二个二进制 `fastgithub-gui-helper`，以 root LaunchDaemon 运行，独占权限操作。
- **Windows/Linux 看门狗**：同一主二进制以 `--watchdog` 参数启动的轻量进程，无需提权。

## 4. 共享核心设计

### 4.1 状态机

`Off → Starting → On → Stopping → Off`，任意阶段失败进入 `Error(原因)` 并回到 Off 语义（不残留半开状态）。托盘图标：On=彩色、Off=灰色、Starting/Stopping=过渡图标、Error=警告图标。菜单首行显示状态文本（错误时含原因）。

### 4.2 内核管理

- 构建期：`scripts/build-kernel.sh` 从官方仓库 creazyboyone/FastGithub 的固定 tag **v2.1.5**（commit `20ef81111fecc9ebed68a1166aaeb815cdcec8c1`，MIT）下载源码 tarball（仅官方 GitHub codeload，**禁止镜像**），固定并校验 tarball SHA256、校验 tag→commit，retarget 到 **.NET 8** 后按 RID 编译自包含单文件内核。三平台统一此路线。
  - **为何不用官方预编译二进制**：官方 v2.1.5 内置 .NET 7 运行时；.NET 7 在 macOS 15 Sequoia+ 因 AppleCrypto bug 无法生成自签证书（`CreateSelfSigned`/`CopyWithPrivateKey` 抛 “item no longer valid”），且 .NET 7 已停止支持、永不修复（修复仅在 .NET 6.0.34/8.0.10/9.0）。retarget 到 .NET 8（LTS，已修复）后实测 macOS 26.5 正常生成证书并代理。Windows/Linux 虽不受此 bug 影响，为供应链一致亦统一源码编译。
  - retarget 最小改动：`Directory.Build.props` net7.0→net8.0；`FastGithub.csproj` 两个 `Microsoft.Extensions.Hosting.*` 7.0.0-rc*→8.0.0；编译设 `-p:InformationalVersion=2.1.5+source`（`ProductionVersion.Parse` 要求版本号含 `+`）。不改业务逻辑。
  - 依赖 .NET 8 SDK（≥8.0.10，微软官方源安装）。产物未裁剪（避免 trimming 破坏反射/DI），osx-x64 约 49MB。
- macOS 内核为 osx-x64（经 Rosetta 运行；上游无 osx-arm64 源码构建差异，x64 已验证）。
- 安装包内置内核；应用首次运行释放到用户数据目录（可写，供生成 cacert）。释放时一并保留内核包内原始 LICENSE 文件（合规，见 §10）：
  - macOS：`~/Library/Application Support/FastGithubGUI/kernel/`
  - Windows：`%APPDATA%/FastGithubGUI/kernel/`
  - Linux：`~/.local/share/FastGithubGUI/kernel/`
- 内核日志写用户数据目录 `logs/fastgithub.log`。
- 端口就绪判定：TCP 连接 `127.0.0.1:38457` 成功，重试上限 15 秒。

### 4.3 git 配置（无需提权，三平台相同）

开启时（git 不存在则跳过并记日志）：
```
git config --global http.https://github.com.proxy http://127.0.0.1:38457
git config --global http.https://github.com.sslCAInfo <内核 cacert/fastgithub.cer 绝对路径>
```
关闭时移除这两项。开启前记录用户原有值，关闭时还原（与系统代理同策略）。不修改 `http.sslVerify`。

### 4.4 托盘菜单

1. 状态行（禁用项）：加速中 / 已关闭 / 错误：原因
2. 开启加速 / 关闭加速（随状态切换）
3. ——分隔——
4. 重新安装组件（修复后台组件/证书）
5. 卸载…（还原全部更改、移除后台组件与证书；应用本体由用户自行删除）
6. 退出（= 关闭加速并还原后退出）
7. 版本号（禁用项）

## 5. 平台后端

### 5.1 macOS

- **一次性安装**（首次开启时）：`osascript` 弹管理员授权，安装 helper 到 `/Library/PrivilegedHelperTools/fastgithub-gui-helper`，写 `/Library/LaunchDaemons/com.fastgithub.gui.helper.plist`（RunAtLoad + KeepAlive），`launchctl bootstrap system`。此后开关免密。
- **通信**：unix socket `/var/run/fastgithub-gui.sock`，JSON 行协议（enable/disable/status/uninstall）。daemon 用 `getpeereid` 校验对端 uid == 安装者 uid。**长连接**：GUI 保持连接，daemon 检测到 EOF 自动 disable（防断网核心）。
- **enable**：启动内核 → 等端口 → 确保证书已信任（`security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain`，按证书主题名查重）→ 记录当前代理 → 设置代理。
- **代理目标服务**：`route -n get default` 取默认网卡，经 `networksetup -listnetworkserviceorder` 映射到网络服务（如 Wi-Fi）。用 `-getwebproxy/-getsecurewebproxy` 记录原值，`-setwebproxy/-setsecurewebproxy` 设为 `127.0.0.1 38457` 并启用；disable 时按记录还原。
- **状态文件**：`/Library/Application Support/FastGithubGUI/daemon-state.json`（root 所有：安装者 uid、代理备份、"代理归我"标记）。
- **daemon 重启一致性**：KeepAlive 拉起后若无 GUI 连接，回到一致 Off 状态（还原标记代理、清理残留内核进程）。
- **uninstall**：osascript 授权 → bootout + 删 plist/helper → 还原代理 → 停内核 → 删除证书信任。

### 5.2 Windows

- **零管理员目标**：代理写 HKCU（`Internet Settings` 的 ProxyEnable/ProxyServer，备份原值含 ProxyOverride），`InternetSetOption` 通知立即生效；证书装**用户级 Root 存储**（`certutil -user -addstore -f Root`，无需 UAC）。若实测个别程序不认用户级证书，回退方案为一次 UAC 装 Local Machine 存储（实现时实测决定，默认用户级）。
- **顺序**：启动内核（用户权限）→ 等端口 → 装证书 → 设代理 → 配 git。
- **看门狗**：主二进制 `--watchdog <父PID> <状态文件>` 启动，监视父进程；父进程消失且无"正常退出"标记 → 按状态文件还原代理、结束内核。正常退出时应用先还原再写标记退出。
- **状态文件**：`%APPDATA%/FastGithubGUI/state.json`。
- **uninstall**：还原代理 → 停内核 → 删用户级证书（`certutil -user -delstore Root <指纹>`）→ 清看门狗。

### 5.3 Linux

- **GNOME 优先**（`XDG_CURRENT_DESKTOP` 含 GNOME）：gsettings `org.gnome.system.proxy` mode=manual，http/https host=127.0.0.1 port=38457；备份原 mode 与值，disable 还原。无需 root。
- **证书**：`certutil -d sql:$HOME/.pki/nssdb -A -t "C,," -n fastgithub -i <cer>`（Chrome/Chromium 系）。检测 certutil 缺失时提示安装 `libnss3-tools`（README 列各发行版命令）。
- **可选系统 CA**：首次开启尝试 `pkexec` 复制到 `/usr/local/share/ca-certificates/` + `update-ca-certificates`（覆盖 curl 等）；用户取消则仅浏览器 + git 加速，不视为错误。
- **看门狗**：同 Windows 模式（gsettings 还原）。状态文件 `~/.config/FastGithubGUI/state.json`。
- **非 GNOME**：弹手动指引（代理地址 + 证书路径），内核与 git 仍自动配置。
- **托盘依赖**：libayatana-appindicator，AppImage 打包时处理（见风险）。
- **uninstall**：还原 gsettings → 删 NSS 证书 → 可选 pkexec 删系统 CA → 停内核。

## 6. 防断网与崩溃安全（三平台统一语义）

- 正常退出：先 disable 还原，再退出。
- 应用崩溃/被杀：macOS 由 daemon 的长连接 EOF 触发还原；Win/Linux 由看门狗触发还原。
- 后台组件自身重启：回到一致 Off 状态，绝不留下"代理指向死端口"。
- 任何 enable 步骤失败：已做的步骤回滚，不设置代理即不算开启。

## 7. 错误处理

| 场景 | 行为 |
|---|---|
| 端口 38457 被占用 | enable 失败，菜单显示原因，保持 Off |
| 内核启动即崩 | 读日志尾部展示，不设代理 |
| 用户取消授权 | 保持 Off，提示信息 |
| 后台组件缺失/损坏 | 提供"重新安装组件"入口 |
| git 不存在 | 跳过 git 步骤，记日志 |
| 非 GNOME 桌面 | 手动指引，其余自动 |

所有错误写日志（用户数据目录 `logs/`，按大小简单轮转）。

## 8. 安装包与分发

- Tauri bundler：macOS `.dmg`（arm64）、Windows NSIS `.exe`、Linux `.AppImage`。
- 无付费签名：网盘分发后 macOS 有 Gatekeeper 提示（右键打开）、Windows 有 SmartScreen（仍要运行）。README 写明步骤，属免费开源工具常态。
- 每平台安装包内置对应架构官方内核。

## 9. CI 与仓库组织

`.github/workflows/build.yml`：tag `v*` 或手动触发；矩阵 macos-14(arm64) / windows-latest / ubuntu-22.04；步骤：checkout → 装 .NET 8 SDK → `build-kernel.sh` 源码编译内核 → Rust + Tauri CLI → 构建 → 上传产物；tag 时创建 GitHub Release 附三个安装包。用户推送源码后 CI 自动出包，下载安装包放网盘。

```
fastgithub-gui/
  src-tauri/          # Rust：核心 + 三平台后端 + helper bin + tauri.conf.json + 图标
  src/                # 极简静态前端（托盘应用无窗口）
  scripts/            # build-kernel（官方源码 retarget net8 编译 + 校验）
  .github/workflows/  # CI
  docs/               # 本设计文档等
  README.md           # 中文：安装/使用/各平台注意/安全提示绕过/常见问题
  LICENSE             # MIT（本项目）
  THIRD_PARTY_NOTICES.md  # FastGithub MIT 全文与署名等
  .gitignore
```

## 10. 许可证（已核实上游原文）

读取下载包内 LICENSE 原文确认：
- **FastGithub**：MIT，`Copyright (c) 2021 老九`（原 dotnetcore/fastgithub 作者，现由 creazyboyone 维护）。
- **dnscrypt-proxy**（FastGithub 包内捆绑）：ISC，`Copyright (c) 2018-2021, Frank Denis <j at pureftpd dot org>`。

两者均为宽松许可证，允许修改与再分发（网盘/GitHub 均合法），**唯一硬性义务：副本中保留版权声明与许可全文**。上游 README"未在 github 之外渠道发布"是其发布渠道说明，非许可证限制。

合规措施：
- 本项目用 MIT，与上游兼容。
- `THIRD_PARTY_NOTICES.md` 收录 FastGithub MIT 全文（含 2021 老九版权行）与 dnscrypt-proxy ISC 全文；README 署名原仓库与现维护仓库。
- 安装包随内核附带其原始 LICENSE 文件，释放内核时一并释放到内核目录，确保二进制副本携带声明。
- 上游 .NET 运行时（MIT）及主要依赖（Yarp MIT、Serilog Apache-2.0 等）在 NOTICES 列明。
- 本项目依赖：Tauri（MIT/Apache-2.0）、Rust crates 以 MIT/Apache 为主；若 Linux 打包 NSS certutil（MPL-2.0）附其声明。

## 11. 里程碑

- **M1 macOS**：装 Rust（rustup 官方源）→ 脚手架 → 核心 + mac 后端 + helper → 本地构建 → 真机实测（用户输一次密码）→ `.dmg`。
- **M2 Windows**：后端 + 看门狗 → CI 出 `.exe` → 用户真机按清单实测。
- **M3 Linux**：GNOME 后端 + 看门狗 → CI 出 `.AppImage` → 用户真机实测。
- **M4 开源整理**：README/LICENSE/NOTICES/release 工作流 → 交付源码目录，用户自行推 GitHub。

每个里程碑结束向用户汇报并确认后再进入下一个。

## 12. 验证清单

**macOS（本地真机）**
- 开：图标变色；系统代理 = 127.0.0.1:38457；浏览器访问 github.com 正常；`git ls-remote https://github.com/creazyboyone/FastGithub` 走代理且证书验证通过。
- 关：代理还原原值；git 两项配置移除；内核进程退出。
- 强杀应用：代理自动还原。
- 重启 Mac：状态一致 Off，无残留代理。
- 卸载：证书信任、daemon、代理全部清除。

**Windows / Linux（用户真机，CI 出包后给详细清单）**
- 同上的开/关/崩溃/卸载语义；Windows 验证用户级证书被浏览器与 git 接受；Linux 验证 GNOME 代理与 NSS 证书。

## 13. 风险与开放问题

- 上游 release 资产名与校验和公布情况需实现时确认；下载失败 Mac 回退 x64。
- Win/Linux 无法在本机运行测试，依赖用户实测反馈迭代。
- Windows 用户级 Root 证书若个别程序不认，回退一次 UAC 方案。
- Linux 托盘依赖 libayatana-appindicator，AppImage 需打包或提示安装。
- 无签名带来的安全提示靠文档说明。
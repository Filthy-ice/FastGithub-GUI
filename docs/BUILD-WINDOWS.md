# 在 Windows 本地编译 FastGithub GUI

本包是**源码 + 已编译好的 win-x64 内核**（`src-tauri/resources/kernel/`），含 v0.1.1 的"控制台窗口反复闪烁"修复。
内核已内置，**无需安装 .NET、无需下载内核源码**。你只需装 Rust + Node + MSVC 构建工具，然后编译 GUI 本体。

## 一、安装前置（只装一次）

1. **Visual Studio Build Tools 2022**（提供 MSVC 编译器和 Windows SDK）
   - 下载：https://visualstudio.microsoft.com/visual-cpp-build-tools/
   - 安装时勾选工作负载 **"使用 C++ 的桌面开发"**（Desktop development with C++），默认组件即可。
2. **Rust**
   - 下载 https://rustup.rs 的 `rustup-init.exe` 运行，默认选项（stable / MSVC）一路回车。
   - 装完**重开** PowerShell，`rustc --version` 能输出版本即成功。
3. **Node.js LTS**
   - 下载 https://nodejs.org 的 LTS 安装包，默认安装。
   - `node --version`、`npm --version` 能输出即成功。

> 下载慢就开 Clash，并在 PowerShell 里设代理：
> `$env:HTTPS_PROXY="http://127.0.0.1:7890"; $env:HTTP_PROXY="http://127.0.0.1:7890"`
> npm 另设：`npm config set proxy http://127.0.0.1:7890; npm config set https-proxy http://127.0.0.1:7890`

## 二、编译与运行

在本源码目录（含 `package.json` 的那层）打开 PowerShell：

```powershell
npm ci                 # 安装 Tauri CLI（仅第一次需要）
```

**方式 A：快速测试（推荐迭代用）**
```powershell
npm run tauri dev
```
直接编译并运行托盘应用，改代码后重跑即可。第一次编译较久（几分钟），之后增量很快。

**方式 B：生成安装包**
```powershell
npm run tauri build
```
产物（NSIS 安装包）：
`src-tauri\target\release\bundle\nsis\FastGithub GUI_0.1.1_x64-setup.exe`

## 三、日志位置（出问题发我）

- 内核日志：`%APPDATA%\FastGithubGUI\logs\fastgithub.log`
  （资源管理器地址栏粘贴 `%APPDATA%\FastGithubGUI\logs` 回车）
- 编译报错：把 PowerShell 里红色错误整段发我。

## 四、验证修复

启动应用后**不应再有 cmd/终端窗口反复闪烁**。点"开启加速"后：
- 系统代理 = `127.0.0.1:38457`（设置→网络和 Internet→代理）
- 浏览器访问 https://github.com 正常、`git ls-remote https://github.com/creazyboyone/FastGithub` 成功。

更多逐项见 `docs/verification-m2-m3.md`。
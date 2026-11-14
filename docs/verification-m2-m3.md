# M2 / M3 真机验证清单（Windows / Linux）

> 安装包由 CI 出包（推 `v*` 标签触发）。本清单供你在自己的 Windows / Linux 机器上实测。
> macOS 已在本地真机验证（见 `verification-m1.md`）。每项通过打勾，失败把现象/日志反馈给我迭代。
> 日志位置：Windows `%APPDATA%/FastGithub GUI/logs/`；Linux `~/.local/share/FastGithub GUI/logs/`。

## Windows（零管理员）

安装与托盘
- [ ] 运行 `*-setup.exe`，SmartScreen 点“更多信息 → 仍要运行”；按用户安装完成。
- [ ] 托盘只有**一个**图标，左键弹出菜单。

开启加速
- [ ] 点“开启加速”，图标变色，菜单显示“加速中”。
- [ ] 系统代理：设置 → 网络和 Internet → 代理，手动代理 = `127.0.0.1:38457`。
      或 PowerShell：`(Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Internet Settings').ProxyServer`
- [ ] 证书：`certutil -user -store Root` 列表里能看到 FastGithub 的 CA（按指纹）。
- [ ] 浏览器访问 https://github.com 正常（无证书警告）。
- [ ] `git ls-remote https://github.com/creazyboyone/FastGithub` 成功。

关闭加速
- [ ] 点“关闭加速”，代理还原为原值（原来没开代理则 ProxyEnable=0）。
- [ ] git 两项配置移除：`git config --global --get http.https://github.com.proxy` 应为空。
- [ ] 内核进程退出（任务管理器无 fastgithub.exe）。

崩溃保护
- [ ] 开启加速后，用任务管理器**强制结束** FastGithub GUI 进程。
- [ ] 约 1–2 秒内系统代理被看门狗自动还原（不会残留 38457 导致断网）。

卸载
- [ ] 菜单“卸载…”后，`certutil -user -store Root` 中该 CA 消失，代理还原。
- [ ] 删除应用本体（设置 → 应用 → 卸载，或删除安装目录）。

## Linux（GNOME 优先）

安装与托盘
- [ ] `chmod +x FastGithub*.AppImage` 后运行。
- [ ] 托盘显示一个图标；若不显示：`sudo apt install libayatana-appindicator3-1` 后重试。

开启加速（GNOME）
- [ ] 点“开启加速”，图标变色。
- [ ] 代理：`gsettings get org.gnome.system.proxy mode` → `'manual'`；
      `gsettings get org.gnome.system.proxy.http host` → `'127.0.0.1'`，port → `38457`。
- [ ] 证书：`certutil -d sql:$HOME/.pki/nssdb -L` 列表含 `fastgithub`。
- [ ] 浏览器访问 https://github.com 正常。
- [ ] `git ls-remote https://github.com/creazyboyone/FastGithub` 成功。
- [ ] （可选系统 CA）若弹出 pkexec 授权并同意，`curl https://github.com` 也走加速；取消不报错。

非 GNOME 桌面
- [ ] 应用提示手动设置代理；按指引把系统/浏览器代理设为 `127.0.0.1:38457` 后访问正常。

关闭加速
- [ ] 点“关闭加速”，`gsettings get org.gnome.system.proxy mode` 还原原值（如 `'none'`）。
- [ ] git 两项配置移除；内核进程退出。

崩溃保护
- [ ] 开启加速后 `kill -9` 主程序进程，约 1–2 秒内 gsettings 代理被看门狗还原。

卸载
- [ ] 菜单“卸载…”后，NSS 中 `fastgithub` 证书移除；若装过系统 CA，
      `/usr/local/share/ca-certificates/fastgithub.crt` 被删除（pkexec）。
- [ ] 删除 AppImage 文件即可。

## 反馈要点

失败时请附：平台与版本、桌面环境（Linux）、上述对应命令的输出、`logs/fastgithub.log` 末尾几行。
# 第三方声明 / Third-Party Notices

FastGithub GUI 本身以 MIT 许可证发布（见 `LICENSE`）。它打包并再分发了以下第三方组件。
按照这些组件的许可证要求，本文件保留其版权声明与许可全文；安装包内的内核目录也随附原始
`LICENSE` 文件（`resources/kernel/LICENSE` 与 `resources/kernel/dnscrypt-proxy/LICENSE`）。

---

## FastGithub（加速内核）

- 上游仓库：https://github.com/dotnetcore/FastGithub （原作者）
- 现维护仓库：https://github.com/creazyboyone/FastGithub
- 版本：v2.1.5（commit `20ef81111fecc9ebed68a1166aaeb815cdcec8c1`）
- 许可证：MIT

本项目从上述官方源码 retarget 到 .NET 8 自行编译内核（原因见 `scripts/build-kernel.sh`），
未修改其加速逻辑。原文如下：

```
MIT License

Copyright (c) 2021 老九

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

---

## dnscrypt-proxy（随 FastGithub 捆绑的 DNS 组件）

- 上游仓库：https://github.com/DNSCrypt/dnscrypt-proxy
- 作者：Frank Denis
- 许可证：ISC

```
ISC License

Copyright (c) 2018-2021, Frank Denis <j at pureftpd dot org>

Permission to use, copy, modify, and/or distribute this software for any
purpose with or without fee is hereby granted, provided that the above
copyright notice and this permission notice appear in all copies.

THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES WITH
REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY
AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY SPECIAL, DIRECT,
INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES WHATSOEVER RESULTING FROM
LOSS OF USE, DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR
OTHER TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR
PERFORMANCE OF THIS SOFTWARE.
```

---

## 内核运行时与主要依赖

自包含内核还包含以下组件（随 .NET 发布输出分发，各自许可证文本见其官方仓库）：

| 组件 | 许可证 | 版权方 |
| --- | --- | --- |
| .NET Runtime / ASP.NET Core 8 | MIT | Microsoft Corporation |
| Yarp（Yet Another Reverse Proxy） | MIT | Microsoft Corporation |
| Serilog / Serilog.Extensions.Hosting | Apache-2.0 | Serilog Contributors |
| Microsoft.Extensions.* | MIT | Microsoft Corporation |

---

## GUI 框架与构建依赖

| 组件 | 许可证 | 说明 |
| --- | --- | --- |
| Tauri 2 | MIT OR Apache-2.0 | 托盘应用框架 |
| Rust crates（serde、anyhow、sha2、winreg、windows-sys、libc 等） | 多为 MIT / Apache-2.0 | 见各 crate 仓库 |
| NSS `certutil`（Linux） | MPL-2.0 | 仅调用系统已安装的 certutil，未随包分发其代码 |

---

## 商标声明

- “GitHub” 及 GitHub 标志（Octocat / Invertocat）是 GitHub, Inc. 的商标。本项目的应用图标
  仅为示意，与 GitHub, Inc. 无关联、未获其背书或赞助。
- “FastGithub” 名称沿用上游开源项目，版权归其原作者所有。
<div align="center">

<img src="assets/app-icon.svg" alt="tty7" width="88" height="88" />

### tty7 · 客制维护版

**在 tty7 上持续维护 Windows、CMD 与 Cmder 体验优化。**

<sub>会话常驻 · 远程开发 · Coding Agent · 纯 Rust</sub>

<br />

[![CI](https://github.com/cloudy-liu/tty7/actions/workflows/ci.yml/badge.svg)](https://github.com/cloudy-liu/tty7/actions/workflows/ci.yml)
[![客制版本](https://img.shields.io/github/v/release/cloudy-liu/tty7?label=%E5%AE%A2%E5%88%B6%E7%89%88%E6%9C%AC&color=3FDD8C)](https://github.com/cloudy-liu/tty7/releases/latest)
[![平台](https://img.shields.io/badge/%E5%B9%B3%E5%8F%B0-macOS%20%C2%B7%20Windows%20%C2%B7%20Linux-blue)](https://github.com/cloudy-liu/tty7/releases/latest)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue)](LICENSE)

<sub>[English](README.md) · 简体中文</sub>

<br />

<img src="assets/hero.webp" alt="tty7 展示跨仓库常驻运行的 coding agent 会话" width="900" />

</div>

> [!IMPORTANT]
> 这是基于 [l0ng-ai/tty7](https://github.com/l0ng-ai/tty7) 独立维护的 fork。
> tty7 的完整功能、配置、编译方法和通用文档，请直接查看
> [上游 README](https://github.com/l0ng-ai/tty7/blob/main/README.zh-CN.md) 与
> [上游文档](https://github.com/l0ng-ai/tty7/tree/main/docs)。本页只说明这个
> fork 额外维护了什么，以及从哪里下载客制安装包。

## tty7 是什么

tty7 是一个 GPU 渲染的终端工作台。真正持有 shell 和 pane 的是后台
server，而不是窗口，因此关闭应用甚至重启机器后仍能恢复会话。它把本地与
远程终端、原生 SSH、Git 工作流、编辑器级提示符输入，以及 Codex、Claude
Code 等 coding agent 的状态感知放在同一个应用里。

完整产品能力以上游为准。这个 fork 会选择性同步上游，同时独立维护下面的
优化。

## 这个 fork 改了什么

### CMD 与 Cmder 提示符集成

- 为原生 `cmd.exe` 增加提示符边界和工作目录上报，使 tty7 能提供历史影子
  建议、Tab 补全和提示符编辑。
- 通过 Clink 的 `CLINK_PATH` 集成 Cmder，完整保留
  `cmd.exe /K init.bat` 等启动方式中的所有用户参数。
- 原生 CMD 无法上报命令开始时，通过 Windows 进程树判断命令是否仍在运行，
  避免 tty7 的输入层覆盖全屏程序，并在命令结束后重新接管提示符。

### 更可靠的 Windows Agent 识别

- 遍历 pane 的 Windows 进程树，识别位于 CMD、Cmder、脚本包装器和辅助
  进程下面的 coding agent。
- 把 agent 身份绑定到正确的 pane，避免把提示符运行的 Git 命令或 MCP 子进程
  误识别成前台 agent。

### 更清晰的侧边栏

- 分屏时，标签页上的 agent 徽标跟随当前聚焦的 pane。
- 支持从右键菜单重命名仓库/侧边栏分组。

## 下载客制安装包

从 [**cloudy-liu/tty7 Releases**](https://github.com/cloudy-liu/tty7/releases/latest)
下载最新客制版本。

| 平台 | 安装包 | 说明 |
|---|---|---|
| **Windows x86_64** | `…-setup.exe` 或免安装 `….zip` | 这是本 fork 的主要优化平台。当前构建尚未进行商业代码签名，SmartScreen 可能要求确认。 |
| **macOS** | `…-macos-arm64.dmg` 或 `…-macos-x86_64.dmg` | 配置 Apple 公证凭据前使用 ad-hoc 签名，Gatekeeper 可能要求手动确认。 |
| **Linux x86_64** | `….AppImage` 或 `….tar.gz` | AppImage 已打包常用的 X11/Wayland 运行库。 |

Release 同时提供 `checksums.txt`，以及远程工作区需要的无头
`tty7-server` 二进制。

## 版本规则

客制版本沿用上游基础版本号，并追加 `-c`：

```text
上游 26.8.3  →  客制版 26.8.3-c  →  tag v26.8.3-c
```

如果在下一个上游版本前还要继续发布，则依次使用 `-c.1`、`-c.2`。同步到
新的上游基础版本后开启新序列，例如 `26.8.4-c`。

这个 fork 的应用内更新和远程 server 安装都从 `cloudy-liu/tty7` 获取资源；
安装客制版本后，不会在更新时悄悄换回上游二进制。

## 上游项目与完整文档

- [上游仓库与英文 README](https://github.com/l0ng-ai/tty7#readme)
- [上游中文 README](https://github.com/l0ng-ai/tty7/blob/main/README.zh-CN.md)
- [上游完整文档](https://github.com/l0ng-ai/tty7/tree/main/docs)
- [上游 Releases](https://github.com/l0ng-ai/tty7/releases)

客制版本特有的问题请提交到
[本 fork 的 Issues](https://github.com/cloudy-liu/tty7/issues)。通用的 tty7
用法和行为问题，请先查阅上游文档。

## License

与上游一致，采用 Apache-2.0。详见 [LICENSE](LICENSE)。

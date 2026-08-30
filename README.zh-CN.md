<div align="center">

<img src="assets/app-icon.svg" alt="tty7" width="88" height="88" />

### tty7 · 客制维护版

**在 tty7 上持续维护：更深的 Windows shell 集成、原生 shell 历史、
WSL 与 SSH 修复，以及独立的客制发布线。**

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

本节列出的都是相对上游的差异。**上游状态**一列说明每项改动在上游项目的处境：

- **上游已关闭** —— 已向上游提交，但被关闭，因此预计会长期只存在于本 fork。
- **未提交上游** —— 尚未向上游提交。
- **仅限 fork** —— 只在 fork 语境下有意义，不会向上游提交。

| 改动 | 平台 | 上游状态 |
|---|---|---|
| CMD 与 Cmder 提示符上报、补全与提示符编辑 | Windows | 未提交上游 |
| 通过 Windows 进程树识别 coding agent | Windows | 未提交上游 |
| ConPTY win32-input-mode 下的 Shift+Enter | Windows | 未提交上游 |
| pane 内 `ssh` 跳转识别 | Windows | [上游已关闭](https://github.com/l0ng-ai/tty7/pull/739) |
| WSL 账号登录 shell 解析 | Windows · WSL | 未提交上游 |
| 提示符文本可选择与 Windows 路径智能选择 | 全平台 | 未提交上游 |
| 用于搜索和建议的原生 shell 历史 | 全平台 | 未提交上游 |
| 侧边栏分组重命名 | 全平台 | [上游已关闭](https://github.com/l0ng-ai/tty7/pull/735) |
| agent 徽标跟随聚焦的 pane | 全平台 | [上游已关闭](https://github.com/l0ng-ai/tty7/pull/719) |
| Antigravity agent 图标 | 全平台 | 未提交上游 |
| 响铃默认关闭 | 全平台 | fork 默认值 |
| 更新检查不走 GitHub REST API | 全平台 | 仅限 fork |
| 客制 `-c` 发布线与更新通道 | 全平台 | 仅限 fork |

早期有一项 fork 改动——分屏光标的实时聚焦——已被上游接受为
[l0ng-ai/tty7#736](https://github.com/l0ng-ai/tty7/pull/736)，因此不再是差异。

### Windows shell 集成

- 为原生 `cmd.exe` 增加提示符边界和工作目录上报，使 tty7 能提供历史影子
  建议、Tab 补全和提示符编辑。
- 通过 Clink 的 `CLINK_PATH` 集成 Cmder，完整保留
  `cmd.exe /K init.bat` 等启动方式中的所有用户参数。
- 把 Clink 的工作目录上报移到提示符字符串之外，避免过长的路径把提示符本身
  挤出可见区域。
- 原生 CMD 无法上报命令开始时，通过 Windows 进程树判断命令是否仍在运行，
  避免 tty7 的输入层覆盖全屏程序，并在命令结束后重新接管提示符。
- 遍历 pane 的进程树，识别位于 CMD、Cmder、脚本包装器和辅助进程下面的
  coding agent，把 agent 身份绑定到正确的 pane，避免把提示符运行的 Git
  命令或 MCP 子进程误识别成前台 agent。
- 在快照重放和重连之间保持 ConPTY 的 `win32-input-mode` 握手状态，并把带
  修饰键的 Enter 编码为 key event，使 **Shift+Enter** 能真正送达申请了该模式
  的 shell 和 agent，而不是直接提交当前行。

### 提示符选择与原生 shell 历史

- 保持 shell 绘制的提示符文本可选择，并把 Windows 驱动器号和反斜杠路径识别为
  一个完整的智能选择范围。
- 在单行提示符边缘把上下方向键交给正在运行的 shell，使 DOSKEY、PSReadLine、
  readline、zle、fish、当前会话条目、重复项和自定义绑定保持原生行为。
- 为 tty7 的模糊搜索和历史影子建议读取 PSReadLine 与 Clink 历史，同时过滤多行
  片段和 Clink 元数据。

### WSL 与 SSH

- 通过 NSS 解析当前发行版账号的登录 shell，并依次回退到 `/etc/passwd`、继承的
  `$SHELL` 和 `/bin/sh`。
- 避免受 [microsoft/WSL#10718](https://github.com/microsoft/WSL/issues/10718)
  影响的 WSL 版本把 `wsl.exe --exec sh` 的 bootstrap 当作用户 shell 启动。
- 在 Windows 上通过遍历进程树并读取参数，识别在 pane 内启动的 `ssh` 会话，
  随后取消该 pane 的本地 Git 侧边栏分组，并在远程上下文就绪后刷新侧边栏。

### 侧边栏与 agent

- 支持从分组标题的右键菜单重命名仓库/侧边栏分组。提交空名称即可恢复由路径
  派生的标题。
- 让这些自定义名称在配置文件中保持稳定顺序，避免保存任意设置时被重新排列。
- 分屏时，标签页上的 agent 徽标跟随当前聚焦的 pane。
- 内置 Antigravity 品牌标识作为 agent 头像。

### 与上游不同的默认值

- **响铃完全关闭**（`"bell": "none"`）。上游默认是 `"visual"`，即闪烁提示而非
  声音；把 `"bell"` 设回 `"visual"` 即可恢复上游行为，需要声音则设为
  `"audible"` 或 `"both"`。

### 构建与更新

- 从 `github.com` 的 `/releases/latest` 重定向读取 Stable 标签，从
  `nightly.json` 读取 Nightly 版本，不再依赖有速率限制的 REST 接口，
  因此不带 token 也能正常检查更新。

客制发布规则、以及更新如何指向本 fork，见[版本规则](#版本规则)。

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

每个客制版本都会记录相对上一个客制 tag 的准确范围。发布提交和 GitHub Release
说明会统计合入的分支线与 Git 提交数，逐笔说明提交解决的问题，并引用对应的 fork
PR、上游 PR 或外部 issue。补丁等价的历史同步会单独说明，不会被写成新增功能。

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

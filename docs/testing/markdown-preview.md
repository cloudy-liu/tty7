# Markdown 阅读主题实现验证

验证日期：2026-09-13。当前执行环境为 Windows，Rust 目标为 `x86_64-pc-windows-msvc`。

## 实现与依赖

- 应用基线：`7e66d9ad531135719895c980b5a0ee23e830c477`，工作分支 `review/markdown-typora`。
- 组件基线：`l0ng-ai/gpui-component` 的 `070d1a28cec5130bf7c4c7895683595d488155b8`。
- 组件交付：[cloudy-liu/gpui-component@2ebbed0674739b5fc56f71bef1b4a436b08321e7](https://github.com/cloudy-liu/gpui-component/commit/2ebbed0674739b5fc56f71bef1b4a436b08321e7)，分支 `feat/markdown-reading-themes`。
- 组件评审：[cloudy-liu/gpui-component#1](https://github.com/cloudy-liu/gpui-component/pull/1)，目标分支为该 fork 的 `tty7`。
- `gpui-component`、`gpui-component-assets` 和传递依赖 `gpui-component-macros` 均来自上述提交。应用 manifest 与锁文件不依赖临时组件目录。
- 应用仍使用原有锁定的 GPUI 定制版本 `l0ng-ai/zed@d99a40a5`。组件自身的独立测试使用其原有上游 GPUI pin；应用测试验证定制 GPUI 下的集成。

## 自动化结果

| 验证 | 命令 | 结果 |
|---|---|---|
| 应用全量回归（含 16 项 Markdown 用例） | `cargo test --locked -p tty7 --bin tty7-app` | 1,498 项通过，2 项原有测试忽略 |
| 原生文本组件 | 在组件仓库运行 `cargo test -p gpui-component --lib --features tree-sitter-languages text::` | 57 项通过 |
| 组件库回归 | 在组件仓库运行 `cargo test -p gpui-component --lib --features tree-sitter-languages` | 287 项通过 |
| Host 边界 | `bash .github/scripts/check-host-boundary.sh` | 通过，扫描 91 个 UI/terminal 文件 |
| Windows 最终锁定构建 | `cargo build --locked` | 通过，已从维护 fork 获取组件，最终程序由实际 worktree 构建 |
| 锁定依赖的全量测试 | `cargo test --locked` | 通过；最后的监听修正后再次运行上述应用全量回归 |
| 更新器 | `cargo test --locked --features updater --bin tty7-updater` | 17 项通过 |
| 格式与差异检查 | `cargo fmt --check`、`git diff --check` | 通过 |
| 文档导航 | 解析 `docs/docs.json` | 通过 |
| 隔离源码锁定构建 | `cargo build --locked --manifest-path <隔离目录>/Cargo.toml --target-dir <worktree>/target` | 通过，包含最后的监听修正 |

新增应用测试沿用 GPUI 测试窗口和空 pane。实际文件打开、焦点与键盘输入、编辑撤销、预览未保存内容、保存和关闭均经过应用入口。主题测试使用临时目录中的实际 YAML，验证默认值补全、精确字段错误、重复 ID、损坏与修复、选择保留、同 ID 样式修订，以及字号变化后的段落定位。

监听回归测试分别验证 Markdown、应用配置及应用主题文件事件的分类，以及实际 GPUI 深浅模式。保存 Markdown 主题只刷新阅读主题注册表，保留正在试用的应用配色；配置事件仍能更新应用主题和快捷键。防抖期间合并事件，丢弃过期快照时保留尚未应用的配置变更。

资源测试仅替换 Host I/O，保留实际编辑器、渲染器和异步适配器。两个 Host 在相同文档路径提供不同内容，验证图片从文档所属 Host 加载，且关闭文档后到达的图片结果不会进入另一篇文档。文档链接测试验证新开与已打开文档的标题定位和复用。

组件测试覆盖局部样式与默认视图隔离、同名主题高亮更新、五种提示块及误识别反例、Unicode 重复标题、README 跨段落 HTML 对齐、行内强调与链接图片、图文混排中的硬换行，以及既有的文本选择和滚动布局回归。

全工作区测试还包含 CLI 的 138 项测试和 core 的 1,159 项测试（另有 4 项原有忽略），服务端及集成测试通过。平台条件未满足而执行 0 项的测试不计入 Windows 已验证能力。

隔离源码位于 `C:/Users/cloudy/AppData/Local/Temp/tty7-markdown-verify-620fcd1d`，由基线源码和本次变更组成，不包含相邻的组件 checkout。构建复用现有 target 与 Cargo 缓存，确认 manifest、锁文件和源码无需本机路径依赖；这项验证不等同于空缓存构建或三平台 CI。

## CI 回归修复

2026-09-14，[首次 PR CI](https://github.com/cloudy-liu/tty7/actions/runs/34766823523) 的 Windows 构建与测试通过，macOS 和 Linux 各有一项测试失败。[定向诊断](https://github.com/cloudy-liu/tty7/actions/runs/34791560680) 复现并区分了两处原因：

- macOS 的 Markdown 测试在首帧测量宽度后、宽版布局生效前建立选区。后续布局改变正文宽度，选区在切换主题之前已经清空。测试现在先完成测量与布局，再选择和滚动；通过应用入口从浅色切到深色，并在切换前后及主题文件修复重载后检查完整选区和滚动位置。
- Linux 的终端测试同样在 `fork/main` 的 [CI](https://github.com/cloudy-liu/tty7/actions/runs/34766788704) 失败。提示符状态先于 vi 模式和结束标记到达时，渲染把暂存输入过早转入内置编辑器，导致 shell 没收到 `ls`。现在等到提示符结束标记再决定是否交给内置编辑器。回归测试主动在两批消息之间绘制一帧，保留输入送达及无残留重放的断言。

Markdown 组件依赖无需更改。最新三平台全量结果见 [cloudy-liu/tty7#30 的检查页](https://github.com/cloudy-liu/tty7/pull/30/checks)；上述诊断运行仅用于定位和重复验证，不代替完整 CI。

## 尚待环境验收

以下项目没有在本次环境中执行，不计为已通过：

- Paperglow 深浅配色的原生截图对照、宽窄阅读区的视觉质量，以及 Windows 100%、125%、150%、200% DPI 抽样。
- 实际设置界面操作、应用主题选择器预览与取消、操作系统文件监听触发的主题重载，以及退出重启后的界面检查。
- 真实 SSH 和 SFTP 连接中的默认预览、相对图片和文件链接。
- 接近 4 MiB 上限文档的实际滚动性能。

本次会话没有可调用的 Windows 原生窗口控制运行时；当前 GPUI Windows 后端也未提供测试截图接口。自动化测试使用原生布局和文字整形，但没有生成或声称验证产品截图。

## 人工验收入口

使用 [Markdown 综合样例](../examples/markdown-preview.md) 和仓库的 `README.md`、`README.zh-CN.md`。在设置的外观页选择 Markdown 阅读主题，打开主题目录并放入 [Blue Paper 完整主题](../examples/markdown-themes/blue-paper.yaml)。

先分别查看深浅模式和宽窄文档列，再检查编辑往返、选择复制、长代码与宽表的独立横向滚动。将样例主题的颜色或字号改动并保存，检查已经打开的文档是否刷新且保留阅读位置；随后验证损坏、删除、修复和重启回退。最后在 SSH/SFTP 文档中重复资源与导航流程。

格式与操作说明见 [Markdown 阅读主题](../customization/markdown-themes.mdx)，完整完成判定见 [规格](../specs/markdown-paperglow-preview.md)。

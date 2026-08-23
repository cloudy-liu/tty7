<div align="center">

<img src="assets/app-icon.svg" alt="tty7" width="88" height="88" />

### tty7 · Custom Fork

**A maintained tty7 fork with additional Windows, CMD, and Cmder fixes.**

<sub>Persistent terminal sessions · remote work · coding agents · pure Rust</sub>

<br />

[![CI](https://github.com/cloudy-liu/tty7/actions/workflows/ci.yml/badge.svg)](https://github.com/cloudy-liu/tty7/actions/workflows/ci.yml)
[![Custom release](https://img.shields.io/github/v/release/cloudy-liu/tty7?label=custom%20release&color=3FDD8C)](https://github.com/cloudy-liu/tty7/releases/latest)
[![Platforms](https://img.shields.io/badge/platforms-macOS%20%C2%B7%20Windows%20%C2%B7%20Linux-blue)](https://github.com/cloudy-liu/tty7/releases/latest)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue)](LICENSE)

<sub>English · [简体中文](README.zh-CN.md)</sub>

<br />

<img src="assets/hero.webp" alt="tty7 showing persistent coding-agent sessions across repositories" width="900" />

</div>

> [!IMPORTANT]
> This is an independently maintained fork of [l0ng-ai/tty7](https://github.com/l0ng-ai/tty7).
> For tty7's complete feature list, configuration, build instructions, and
> general documentation, use the [upstream README](https://github.com/l0ng-ai/tty7#readme)
> and [upstream docs](https://github.com/l0ng-ai/tty7/tree/main/docs). This page
> focuses on what this fork changes and where to download its custom builds.

## What tty7 is

tty7 is a GPU-rendered terminal workbench whose background server owns shells
and panes independently of the window. Sessions survive closing the app and can
be resumed after a reboot. It combines local and remote terminals, native SSH,
Git workflows, editor-grade prompt input, and awareness of coding agents such
as Codex and Claude Code.

The upstream project is the source of truth for the product's full behavior.
This fork selectively follows upstream while maintaining the changes below.

## What this fork changes

### CMD and Cmder prompt integration

- Adds prompt-boundary and working-directory reports for stock `cmd.exe`, so
  tty7 can provide ghost suggestions, Tab completion, and prompt editing.
- Integrates Cmder through Clink's `CLINK_PATH`, preserving every user-supplied
  argument in launches such as `cmd.exe /K init.bat`.
- Uses the Windows process tree when bare CMD cannot report command start, so a
  running full-screen program owns its input line and tty7 re-arms the prompt
  editor only when the command has finished.

### Better Windows agent detection

- Detects coding agents below CMD, Cmder, wrappers, and helper processes by
  walking the pane's Windows process tree.
- Keeps agent identity attached to the correct pane instead of mistaking prompt
  helpers or MCP child processes for the foreground agent.

### Sidebar clarity

- Makes tab agent badges follow the focused pane in a split.
- Supports renaming repository/sidebar groups from the context menu.

## Download the custom build

Download the newest fork-maintained build from
[**cloudy-liu/tty7 Releases**](https://github.com/cloudy-liu/tty7/releases/latest).

| Platform | Assets | Notes |
|---|---|---|
| **Windows x86_64** | `…-setup.exe` or portable `….zip` | The primary target for this fork's CMD/Cmder fixes. Builds are currently unsigned, so SmartScreen may ask for confirmation. |
| **macOS** | `…-macos-arm64.dmg` or `…-macos-x86_64.dmg` | Fork builds are ad-hoc signed until Apple notarization credentials are configured. Gatekeeper may require manual confirmation. |
| **Linux x86_64** | `….AppImage` or `….tar.gz` | The AppImage bundles the usual X11/Wayland runtime libraries. |

Release assets also include `checksums.txt` and the headless `tty7-server`
binaries used by remote workspaces.

## Versioning

Custom releases keep the upstream base version and append `-c`:

```text
upstream 26.8.3  →  custom 26.8.3-c  →  tag v26.8.3-c
```

If another custom release is needed before the next upstream version, it uses
`-c.1`, `-c.2`, and so on. A later upstream base starts a new series, for
example `26.8.4-c`.

The application updater and remote-server installer in this fork read releases
from `cloudy-liu/tty7`; installing a custom build will not silently switch back
to an upstream binary.

## Upstream documentation

- [Upstream repository and full English README](https://github.com/l0ng-ai/tty7#readme)
- [Upstream Chinese README](https://github.com/l0ng-ai/tty7/blob/main/README.zh-CN.md)
- [Upstream documentation](https://github.com/l0ng-ai/tty7/tree/main/docs)
- [Upstream releases](https://github.com/l0ng-ai/tty7/releases)

Please report problems specific to this custom build in
[this fork's issue tracker](https://github.com/cloudy-liu/tty7/issues). For
general tty7 usage and behavior, consult the upstream documentation first.

## License

Apache-2.0, matching upstream. See [LICENSE](LICENSE).

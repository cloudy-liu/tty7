<div align="center">

<img src="assets/app-icon.svg" alt="tty7" width="88" height="88" />

### tty7 · Custom Fork

**A maintained tty7 fork: deeper Windows shell integration, native shell
history, WSL and SSH fixes, and its own release line.**

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

Everything in this section is a difference from upstream. The **Upstream**
column records where each change stands with the upstream project:

- **Declined** — proposed upstream and closed there, so it is expected to stay
  fork-only.
- **Not submitted** — not yet proposed upstream.
- **Fork-specific** — only meaningful in a fork, so it will not be proposed.

| Change | Platform | Upstream |
|---|---|---|
| CMD and Cmder prompt reporting, completion, and prompt editing | Windows | Not submitted |
| Coding-agent detection through the Windows process tree | Windows | Not submitted |
| Shift+Enter under ConPTY win32-input-mode | Windows | Not submitted |
| In-pane `ssh` hop detection | Windows | [Declined](https://github.com/l0ng-ai/tty7/pull/739) |
| WSL account login-shell resolution | Windows · WSL | Not submitted |
| Selectable prompt text and Windows path smart-selection | All | Not submitted |
| Native shell history for search and suggestions | All | Not submitted |
| Sidebar group renaming | All | [Declined](https://github.com/l0ng-ai/tty7/pull/735) |
| Agent badges follow the focused pane | All | [Declined](https://github.com/l0ng-ai/tty7/pull/719) |
| Antigravity agent icon | All | Not submitted |
| Bell off by default | All | Fork default |
| Update checks without the GitHub REST API | All | Fork-specific |
| Custom `-c` release line and update channel | All | Fork-specific |

One earlier fork change — live focus for split-pane cursors — was accepted
upstream as [l0ng-ai/tty7#736](https://github.com/l0ng-ai/tty7/pull/736), so it
is no longer a difference.

### Windows shell integration

- Adds prompt-boundary and working-directory reports for stock `cmd.exe`, so
  tty7 can provide ghost suggestions, Tab completion, and prompt editing.
- Integrates Cmder through Clink's `CLINK_PATH`, preserving every user-supplied
  argument in launches such as `cmd.exe /K init.bat`.
- Emits the Clink working-directory report outside the prompt string, so a long
  path no longer pushes the prompt itself out of view.
- Uses the Windows process tree when bare CMD cannot report command start, so a
  running full-screen program owns its input line and tty7 re-arms the prompt
  editor only when the command has finished.
- Detects coding agents below CMD, Cmder, wrappers, and helper processes by
  walking the pane's process tree, keeping agent identity on the correct pane
  instead of mistaking prompt helpers or MCP child processes for the foreground
  agent.
- Latches ConPTY's `win32-input-mode` handshake across snapshot replay and
  reconnect and encodes modified Enter as a key event, so **Shift+Enter**
  reaches the shells and agents that ask for it instead of submitting the line.

### Prompt selection and native shell history

- Keeps shell-rendered prompt text selectable and treats Windows drive-letter
  and backslash paths as one smart-selection range.
- Hands Up and Down to the running shell at the edge of a one-line prompt, so
  DOSKEY, PSReadLine, readline, zle, fish, current-session entries, duplicates,
  and custom bindings keep their native behavior.
- Reads PSReadLine and Clink history files for tty7's fuzzy search and ghost
  suggestions without exposing multiline fragments or Clink metadata.

### WSL and SSH

- Resolves the current distro account's login shell through NSS, with
  `/etc/passwd`, inherited `$SHELL`, and `/bin/sh` fallbacks.
- Avoids starting the `wsl.exe --exec sh` bootstrap as the user's shell on WSL
  releases affected by [microsoft/WSL#10718](https://github.com/microsoft/WSL/issues/10718).
- Recognizes an `ssh` session started inside a pane on Windows by walking the
  process tree and reading its arguments, then drops local Git sidebar grouping
  for that pane and refreshes the sidebar once the remote context arrives.

### Sidebar and agents

- Renames repository and sidebar groups from the group header's context menu.
  Submitting an empty name restores the title derived from the path.
- Keeps those custom names in a stable order in the config file, so saving any
  setting does not reshuffle them.
- Makes tab agent badges follow the focused pane in a split.
- Ships the Antigravity brand mark for agent avatars.

### Defaults that differ from upstream

- **The bell is off** (`"bell": "none"`). Upstream defaults to `"visual"` — a
  flash rather than a sound — so set `"bell"` back to `"visual"` to restore
  upstream's behavior, or to `"audible"` or `"both"` if you want the sound.

### Builds and updates

- Reads the Stable tag from the `github.com` `/releases/latest` redirect and the
  Nightly version from `nightly.json`, rather than the rate-limited REST
  catalog, so update checks keep working without a token.

See [Versioning](#versioning) for the custom release scheme and how the updater
is pointed at this fork.

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

Each custom release records the exact range from the previous custom tag. Its
commit message and GitHub release notes count the integrated branch lines and
Git commits, explain every commit, and link the relevant fork PR, upstream PR,
or external issue. Patch-equivalent history syncs are called out separately so
they are not presented as new behavior.

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

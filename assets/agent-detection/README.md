# Agent screen-detection manifests

These TOML files come from [herdr](https://github.com/herdrdev/herdr)
(`src/detect/manifests/`, commit `c411883ec639`), licensed under the Apache
License 2.0. They are copied without changes, so updating them means copying
them again. `copilot.toml` is herdr's `github-copilot.toml`, renamed after
tty7's slug.

Each file lists rules that classify a coding agent's pane as `working`,
`blocked` or `idle`, based on what the agent's TUI is showing at the bottom of
the screen. `src/terminal/screen_status.rs` evaluates them the way herdr's
`src/detect/manifest.rs` does.

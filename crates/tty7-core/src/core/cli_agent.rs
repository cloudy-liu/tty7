use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CLIAgent {
    Claude,
    Codex,
    Gemini,
    Aider,
    Amp,
    OpenCode,
    Copilot,
    Cursor,
    Goose,
    Droid,
    Pi,
    Auggie,
    Hermes,
    Vibe,
    Antigravity,
    Grok,
    Qwen,
    OhMyPi,
    Kimi,
}

impl CLIAgent {
    pub const ALL: [CLIAgent; 19] = [
        CLIAgent::Claude,
        CLIAgent::Codex,
        CLIAgent::Gemini,
        CLIAgent::Aider,
        CLIAgent::Amp,
        CLIAgent::OpenCode,
        CLIAgent::Copilot,
        CLIAgent::Cursor,
        CLIAgent::Goose,
        CLIAgent::Droid,
        CLIAgent::Pi,
        CLIAgent::Auggie,
        CLIAgent::Hermes,
        CLIAgent::Vibe,
        CLIAgent::Antigravity,
        CLIAgent::Grok,
        CLIAgent::Qwen,
        CLIAgent::OhMyPi,
        CLIAgent::Kimi,
    ];

    fn aliases(self) -> &'static [&'static str] {
        match self {
            CLIAgent::Claude => &["claude", "claude-code"],
            CLIAgent::Codex => &["codex", "codex-cli"],
            CLIAgent::Gemini => &["gemini", "gemini-cli"],
            CLIAgent::Aider => &["aider", "aider-chat"],
            CLIAgent::Amp => &["amp"],
            CLIAgent::OpenCode => &["opencode"],
            CLIAgent::Copilot => &["copilot"],
            CLIAgent::Cursor => &["cursor-agent"],
            CLIAgent::Goose => &["goose"],
            CLIAgent::Droid => &["droid"],
            CLIAgent::Pi => &["pi"],
            CLIAgent::Auggie => &["auggie"],
            CLIAgent::Hermes => &["hermes"],
            CLIAgent::Vibe => &["vibe", "vibe-acp"],
            // `agy` only. The `antigravity` binary the IDE installs is a
            // launcher shim in the shape of VS Code's `code`, not the terminal
            // agent — and the name also collides with `python3 -m antigravity`,
            // the standard way to trigger Python's own easter egg.
            CLIAgent::Antigravity => &["agy"],
            CLIAgent::Grok => &["grok"],
            CLIAgent::Qwen => &["qwen", "qwen-code"],
            // Oh My Pi is a fork of Pi, but it ships one binary of its own and
            // never installs a `pi`, so the two names stay disjoint.
            CLIAgent::OhMyPi => &["omp"],
            // Both the standalone Kimi Code CLI and the legacy open-source
            // kimi-cli install a `kimi` — same vendor, same brand, so one
            // detection covers them. Only the standalone one has hooks.
            CLIAgent::Kimi => &["kimi", "kimi-code"],
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            CLIAgent::Claude => "claude",
            CLIAgent::Codex => "codex",
            CLIAgent::Gemini => "gemini",
            CLIAgent::Aider => "aider",
            CLIAgent::Amp => "amp",
            CLIAgent::OpenCode => "opencode",
            CLIAgent::Copilot => "copilot",
            CLIAgent::Cursor => "cursor",
            CLIAgent::Goose => "goose",
            CLIAgent::Droid => "droid",
            CLIAgent::Pi => "pi",
            CLIAgent::Auggie => "auggie",
            CLIAgent::Hermes => "hermes",
            CLIAgent::Vibe => "vibe",
            CLIAgent::Antigravity => "antigravity",
            CLIAgent::Grok => "grok",
            CLIAgent::Qwen => "qwen",
            CLIAgent::OhMyPi => "omp",
            CLIAgent::Kimi => "kimi",
        }
    }

    pub fn from_slug(name: &str) -> Option<CLIAgent> {
        let name = name.trim().to_ascii_lowercase();
        CLIAgent::ALL.into_iter().find(|a| a.slug() == name)
    }

    pub fn display_name(self) -> &'static str {
        match self {
            CLIAgent::Claude => "Claude Code",
            CLIAgent::Codex => "Codex",
            CLIAgent::Gemini => "Gemini",
            CLIAgent::Aider => "Aider",
            CLIAgent::Amp => "Amp",
            CLIAgent::OpenCode => "OpenCode",
            CLIAgent::Copilot => "Copilot",
            CLIAgent::Cursor => "Cursor",
            CLIAgent::Goose => "Goose",
            CLIAgent::Droid => "Droid",
            CLIAgent::Pi => "Pi",
            CLIAgent::Auggie => "Auggie",
            CLIAgent::Hermes => "Hermes",
            CLIAgent::Vibe => "Vibe",
            CLIAgent::Antigravity => "Antigravity",
            CLIAgent::Grok => "Grok",
            CLIAgent::Qwen => "Qwen Code",
            CLIAgent::OhMyPi => "Oh My Pi",
            CLIAgent::Kimi => "Kimi Code",
        }
    }

    /// A plain interactive launch, with no prompt, history selector or fork.
    /// Reopening it is only valid while the pane has received no agent input.
    pub fn unstarted_command(self, argv: &[String]) -> Option<String> {
        let args = self.invocation_args(argv)?;
        let (program, args) = match (self, args.first().map(String::as_str)) {
            (Self::Goose, Some("session" | "s")) => ("goose session", &args[1..]),
            (Self::Hermes, Some("chat")) => ("hermes chat", &args[1..]),
            _ => (self.aliases()[0], args),
        };
        if !unambiguous_resume_options(args) || self.opts_out_of_sessions(argv) {
            return None;
        }
        Some(format!(
            "{program}{}",
            self.session_command_flags("unstarted", Some(argv))?
        ))
    }

    pub fn resume_command(
        self,
        session_id: &str,
        launch_argv: Option<&[String]>,
    ) -> Option<String> {
        if launch_argv.is_some_and(|argv| self.opts_out_of_sessions(argv)) {
            return None;
        }
        if self == CLIAgent::Aider {
            let history = quoted_history_path(session_id)?;
            let flags = self.session_command_flags("history", launch_argv)?;
            return Some(format!(
                "aider{flags} --chat-history-file {history} --restore-chat-history"
            ));
        }
        let flags = self.session_command_flags(session_id, launch_argv)?;
        match self {
            CLIAgent::Claude => Some(format!("claude{flags} --resume {session_id}")),
            CLIAgent::Codex => Some(format!("codex resume {session_id}{flags}")),
            CLIAgent::Gemini => Some(format!("gemini{flags} --resume {session_id}")),
            CLIAgent::OpenCode => Some(format!("opencode{flags} --session {session_id}")),
            CLIAgent::Amp => Some(format!("amp threads continue {session_id}{flags}")),
            CLIAgent::Auggie => Some(format!("auggie{flags} --resume {session_id}")),
            CLIAgent::Hermes => Some(format!("hermes chat{flags} --resume {session_id}")),
            CLIAgent::Qwen => Some(format!("qwen{flags} --resume {session_id}")),
            CLIAgent::Goose => Some(format!(
                "goose session{flags} --resume --session-id {session_id}"
            )),
            CLIAgent::Vibe => Some(format!("vibe{flags} --resume {session_id}")),
            CLIAgent::Antigravity => Some(format!("agy{flags} --conversation {session_id}")),
            CLIAgent::Cursor => Some(format!("cursor-agent{flags} --resume {session_id}")),
            CLIAgent::Droid => Some(format!("droid{flags} --resume {session_id}")),
            CLIAgent::Copilot => Some(format!("copilot{flags} --resume {session_id}")),
            CLIAgent::Grok => Some(format!("grok{flags} --resume {session_id}")),
            CLIAgent::Pi => Some(format!("pi{flags} --session {session_id}")),
            CLIAgent::OhMyPi => Some(format!("omp{flags} --resume {session_id}")),
            CLIAgent::Kimi => Some(format!("kimi{flags} --session {session_id}")),
            _ => None,
        }
    }

    /// An explicit conversation id in a captured resume command. Hooks remain
    /// authoritative; this fills the gap before a resumed agent sends one.
    /// Aider's target is its history file. Other agents need a complete native
    /// id: names, indexes, `latest`, and a fork's parent are not exact targets.
    pub fn resumed_session_id(self, argv: &[String]) -> Option<&str> {
        if !matches!(self, CLIAgent::Codex | CLIAgent::Claude) {
            return self.canonical_resumed_session_id(argv);
        }
        let args = self.invocation_args(argv)?;
        let id = match self {
            CLIAgent::Codex => {
                let args = &args[..args.iter().position(|s| s == "--").unwrap_or(args.len())];
                if args.first()?.as_str() != "resume"
                    || args.iter().any(|s| s.split('=').next() == Some("--last"))
                {
                    return None;
                }
                args.get(1)?.as_str()
            }
            CLIAgent::Claude => {
                let mut id = None;
                let mut args = args.iter().map(String::as_str);
                while let Some(arg) = args.next() {
                    let (flag, value) = arg
                        .split_once('=')
                        .map_or((arg, None), |(f, v)| (f, Some(v)));
                    match flag {
                        "--" => break,
                        "--fork-session" | "--continue" | "-c" | "--session-id" | "--from-pr" => {
                            return None;
                        }
                        "--resume" | "-r" => {
                            if id.is_some() || (flag == "-r" && value.is_some()) {
                                return None;
                            }
                            id = Some(value.or_else(|| args.next())?);
                        }
                        "--dangerously-skip-permissions"
                        | "--allow-dangerously-skip-permissions"
                        | "--verbose"
                        | "--strict-mcp-config"
                        | "--disable-slash-commands"
                        | "--ide"
                        | "--chrome"
                        | "--no-chrome"
                            if value.is_none() => {}
                        "--model"
                        | "--fallback-model"
                        | "--effort"
                        | "--permission-mode"
                        | "--permission-prompt-tool"
                        | "--agent"
                        | "--agents"
                        | "--system-prompt"
                        | "--append-system-prompt"
                        | "--system-prompt-file"
                        | "--append-system-prompt-file"
                        | "--settings"
                        | "--setting-sources"
                        | "--mcp-config"
                        | "--allowedTools"
                        | "--allowed-tools"
                        | "--disallowedTools"
                        | "--disallowed-tools"
                        | "--tools"
                        | "--add-dir"
                        | "--plugin-dir" => {
                            // Required values can themselves look like --resume.
                            // Unknown/variadic syntax stays with the hook fallback.
                            if value.is_none() {
                                args.next()?;
                            }
                        }
                        _ => return None,
                    }
                }
                id?
            }
            _ => unreachable!(),
        };
        self.replay_flags(argv)?;
        (id.len() == 36 && uuid::Uuid::try_parse(id).is_ok()).then_some(id)
    }

    fn canonical_resumed_session_id(self, argv: &[String]) -> Option<&str> {
        let args = self.invocation_args(argv)?;
        let (id, options) = match self {
            CLIAgent::Amp if args.first()?.as_str() == "threads" => {
                if args.get(1)?.as_str() != "continue" {
                    return None;
                }
                (args.get(2)?.as_str(), &args[3..])
            }
            CLIAgent::Aider => {
                let [options @ .., flag, id, restore] = args else {
                    return None;
                };
                if flag != "--chat-history-file" || restore != "--restore-chat-history" {
                    return None;
                }
                (id.as_str(), options)
            }
            _ => {
                let [options @ .., flag, id] = args else {
                    return None;
                };
                let selector = match self {
                    CLIAgent::OpenCode | CLIAgent::Pi | CLIAgent::Kimi => "--session",
                    CLIAgent::Antigravity => "--conversation",
                    CLIAgent::Goose => "--session-id",
                    _ => "--resume",
                };
                if flag != selector {
                    return None;
                }
                let options = match self {
                    CLIAgent::Goose => {
                        let [session, options @ .., resume] = options else {
                            return None;
                        };
                        if session != "session" || resume != "--resume" {
                            return None;
                        }
                        options
                    }
                    CLIAgent::Hermes => {
                        let [chat, options @ ..] = options else {
                            return None;
                        };
                        if chat != "chat" {
                            return None;
                        }
                        options
                    }
                    _ => options,
                };
                (id.as_str(), options)
            }
        };
        // Be conservative about an option consuming a following --resume as
        // its value. Unknown syntax belongs in the user's selection prompt.
        if !unambiguous_resume_options(options) || !self.resumes_session(id, argv) {
            return None;
        }
        let uuid = |s: &str| s.len() == 36 && uuid::Uuid::try_parse(s).is_ok();
        let exact = uuid(id)
            || match self {
                CLIAgent::Amp => id.strip_prefix("T-").is_some_and(uuid),
                CLIAgent::OpenCode => id.strip_prefix("ses_").is_some_and(|s| {
                    s.len() >= 20 && s.len() <= 40 && s.bytes().all(|c| c.is_ascii_alphanumeric())
                }),
                CLIAgent::Goose => id.split_once('_').is_some_and(|(date, number)| {
                    date.len() == 8
                        && date.bytes().all(|c| c.is_ascii_digit())
                        && !number.is_empty()
                        && number.bytes().all(|c| c.is_ascii_digit())
                }),
                CLIAgent::Aider => quoted_history_path(id).is_some(),
                _ => false,
            };
        exact.then_some(id)
    }

    /// Whether this is the resume command tty7 would queue for an already
    /// known id. Unlike recovering an unknown id, this works for every agent
    /// with a resume command, including agents whose ids are not UUIDs.
    pub fn resumes_session(self, session_id: &str, argv: &[String]) -> bool {
        let Some(args) = self.invocation_args(argv) else {
            return false;
        };
        let Some(command) = self.resume_command(session_id, Some(argv)) else {
            return false;
        };
        args == &command_argv(&command)[1..]
    }

    fn invocation_args(self, argv: &[String]) -> Option<&[String]> {
        let argv = &argv[argv.iter().take_while(|t| is_env_assignment(t)).count()..];
        let (program, args) = argv.split_first()?;
        let program = program.to_ascii_lowercase();
        if Self::match_token(base_stem(&program)) == Some(self) {
            Some(args)
        } else if is_interpreter(base_stem(&program)) {
            let named = args.iter().position(|arg| {
                arg.split(['/', '\\']).any(|part| {
                    Self::match_token(base_stem(&part.to_ascii_lowercase())) == Some(self)
                })
            })?;
            Some(&args[named + 1..])
        } else {
            None
        }
    }

    pub fn opts_out_of_sessions(self, argv: &[String]) -> bool {
        let ephemeral: &[&str] = match self {
            CLIAgent::Pi | CLIAgent::OhMyPi => &["--no-session"],
            // "Do not save conversation history" — nothing is persisted, so
            // there is no session left to resume from.
            CLIAgent::Auggie => &["--dont-save-session"],
            // "If false, chat history is not saved and --continue/--resume
            // will not work" — the yargs negation of `--chat-recording`.
            CLIAgent::Qwen => &["--no-chat-recording"],
            _ => &[],
        };
        argv.iter().any(|t| ephemeral.contains(&t.as_str()))
    }

    pub fn fork_command(self, session_id: &str, launch_argv: Option<&[String]>) -> Option<String> {
        if launch_argv.is_some_and(|argv| self.opts_out_of_sessions(argv)) {
            return None;
        }
        let flags = self.session_command_flags(session_id, launch_argv)?;
        match self {
            CLIAgent::Codex => Some(format!("codex fork {session_id}{flags}")),
            CLIAgent::Claude => Some(format!(
                "claude{flags} --resume {session_id} --fork-session"
            )),
            CLIAgent::Grok => Some(format!("grok{flags} --resume {session_id} --fork-session")),
            CLIAgent::OpenCode => Some(format!("opencode{flags} --session {session_id} --fork")),
            CLIAgent::OhMyPi => Some(format!("omp{flags} --fork {session_id}")),
            // Droid forks with a standalone flag rather than resume-plus-a-switch.
            CLIAgent::Droid => Some(format!("droid{flags} --fork {session_id}")),
            // `fork` is missing from `amp threads --help`, but the subcommand is
            // real — `amp threads fork --help` prints its own usage.
            CLIAgent::Amp => Some(format!("amp threads fork {session_id}{flags}")),
            CLIAgent::Qwen => Some(format!("qwen{flags} --resume {session_id} --fork-session")),
            // Goose forks by adding a switch to the same resume invocation.
            CLIAgent::Goose => Some(format!(
                "goose session{flags} --resume --fork --session-id {session_id}"
            )),
            _ => None,
        }
    }

    pub fn fork_label(self) -> Option<&'static str> {
        match self {
            CLIAgent::Claude
            | CLIAgent::Codex
            | CLIAgent::Grok
            | CLIAgent::OpenCode
            | CLIAgent::OhMyPi
            | CLIAgent::Droid
            | CLIAgent::Amp
            | CLIAgent::Qwen
            | CLIAgent::Goose => Some("Fork Session"),
            _ => None,
        }
    }

    fn session_command_flags(
        self,
        session_id: &str,
        launch_argv: Option<&[String]>,
    ) -> Option<String> {
        if session_id.is_empty()
            || session_id.starts_with('-')
            || !session_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.')
        {
            return None;
        }
        Some(
            launch_argv
                .and_then(|argv| self.replay_flags(argv))
                .map(|flags| {
                    flags.iter().fold(String::new(), |mut s, f| {
                        s.push(' ');
                        s.push_str(f);
                        s
                    })
                })
                .unwrap_or_default(),
        )
    }

    fn replay_flags(self, argv: &[String]) -> Option<Vec<String>> {
        let names_self = |token: &str| {
            token.split(['/', '\\']).any(|seg| {
                CLIAgent::match_token(&base_stem(seg).to_ascii_lowercase()) == Some(self)
            })
        };
        let argv = &argv[argv.iter().take_while(|t| is_env_assignment(t)).count()..];
        let named = argv.iter().position(|t| names_self(t))?;
        let mut tail: Vec<&str> = argv[named + 1..].iter().map(String::as_str).collect();

        if self == CLIAgent::Codex && matches!(tail.first(), Some(&"resume") | Some(&"fork")) {
            tail.remove(0);
            if tail.first().is_some_and(|t| !t.starts_with('-')) {
                tail.remove(0);
            }
        }

        // Agents that reach their session through subcommands leave `stale`
        // nothing to drop — `amp threads continue <id>` names the thread with a
        // positional argument, and `goose session --resume` hides the flags one
        // level down. Either way the prefix has to come off here, because the
        // "a bare token must follow a flag" check below would otherwise reject
        // the tail wholesale and take every launch flag down with it. The
        // replacement command spells the subcommand out again itself.
        let (groups, verbs): (&[&str], &[&str]) = match self {
            CLIAgent::Amp => (
                &["threads", "t"],
                &["continue", "c", "fork", "f", "handoff", "h"],
            ),
            CLIAgent::Auggie => (&["session"], &["resume", "continue"]),
            CLIAgent::Goose => (&["session", "s"], &[]),
            CLIAgent::Hermes => (&["chat"], &[]),
            _ => (&[], &[]),
        };
        if tail.first().is_some_and(|t| groups.contains(t)) {
            tail.remove(0);
            if tail.first().is_some_and(|t| verbs.contains(t)) {
                tail.remove(0);
                if tail.first().is_some_and(|t| !t.starts_with('-')) {
                    tail.remove(0);
                }
            }
        }

        let stale: &[&str] = match self {
            CLIAgent::Aider => &[
                "--chat-history-file",
                "--restore-chat-history",
                "--no-restore-chat-history",
            ],
            CLIAgent::Claude => &[
                "--resume",
                "-r",
                "--continue",
                "-c",
                "--session-id",
                "--from-pr",
                "--fork-session",
            ],
            // `--session-id` and `--session-file` name a session too, and Gemini
            // rejects them outright alongside `--resume`.
            CLIAgent::Gemini => &["--resume", "-r", "--session-id", "--session-file"],
            CLIAgent::Cursor => &["--resume", "-r", "--continue"],
            CLIAgent::Copilot | CLIAgent::Auggie | CLIAgent::Hermes => {
                &["--resume", "-r", "--continue", "-c"]
            }
            // `--session-id` names a *new* session and Qwen rejects it
            // alongside `--resume`, so it is as stale as the resume flags.
            CLIAgent::Qwen => &[
                "--resume",
                "-r",
                "--continue",
                "-c",
                "--fork-session",
                "--session-id",
            ],
            CLIAgent::Droid => &["--resume", "-r", "--fork", "--session-id", "-s"],
            // `--session-id`/`--id`, `-n`/`--name` and the legacy `--path` are
            // one mutually-exclusive clap group in Goose; any of them surviving
            // next to the `--session-id` this command appends is a parse error.
            CLIAgent::Goose => &[
                "--resume",
                "-r",
                "--fork",
                "--session-id",
                "--id",
                "--name",
                "-n",
                "--path",
            ],
            CLIAgent::Vibe => &["--resume", "--continue", "-c"],
            CLIAgent::Antigravity => &["--conversation", "--continue", "-c"],
            CLIAgent::OpenCode => &["--session", "-s", "--continue", "-c", "--fork"],
            CLIAgent::Codex => &["--last"],
            CLIAgent::Pi => &[
                "--session",
                "--session-id",
                "--fork",
                "--resume",
                "-r",
                "--continue",
                "-c",
            ],
            // `--resume`, `-r` and `--session` are three spellings of one flag
            // in Oh My Pi; `--session-dir` is a different one and survives.
            CLIAgent::OhMyPi => &["--resume", "-r", "--session", "--fork", "--continue", "-c"],
            // `--resume`/`-r` is Kimi's hidden alias for `--session`/`-S`.
            // `--agent`/`--agent-file` bind the main agent at session creation
            // and Kimi rejects either next to `--session` outright; resuming
            // restores the bound agent by itself, so replaying them would only
            // turn a working resume into a startup error.
            CLIAgent::Kimi => &[
                "--session",
                "-S",
                "--resume",
                "-r",
                "--continue",
                "-c",
                "--agent",
                "--agent-file",
            ],
            CLIAgent::Grok => &[
                "--resume",
                "-r",
                "--load",
                "--continue",
                "-c",
                "--session-id",
                "-s",
                "--fork-session",
                "--worktree",
                "-w",
                "--worktree-ref",
                "--ref",
            ],
            _ => &[],
        };
        let mut i = 0;
        while i < tail.len() {
            let t = tail[i];
            if stale.contains(&t)
                || stale
                    .iter()
                    .any(|f| f.len() > 2 && t.starts_with(&format!("{f}=")))
            {
                tail.remove(i);
                if i < tail.len() && !tail[i].starts_with('-') {
                    tail.remove(i);
                }
            } else {
                i += 1;
            }
        }

        let safe = |t: &str| {
            !t.is_empty()
                && t.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_=./,:@+~".contains(&b))
        };
        if !tail.iter().all(|t| safe(t)) {
            return None;
        }
        let mut prev_was_flag = false;
        for t in &tail {
            let is_flag = t.starts_with('-');
            if !is_flag && !prev_was_flag {
                return None;
            }
            prev_was_flag = is_flag;
        }
        Some(tail.into_iter().map(String::from).collect())
    }

    pub fn accent_rgb(self) -> u32 {
        match self {
            CLIAgent::Claude => 0xD97757,
            CLIAgent::Codex => 0x000000,
            CLIAgent::Gemini => 0x4285F4,
            CLIAgent::Aider => 0x14B014,
            CLIAgent::Amp => 0xF34E3F,
            CLIAgent::OpenCode => 0x6E56CF,
            CLIAgent::Copilot => 0x8957E5,
            CLIAgent::Cursor => 0x9AA0A6,
            CLIAgent::Goose => 0x3ECC5F,
            CLIAgent::Droid => 0xEF6F2E,
            CLIAgent::Pi => 0x0EA5E9,
            CLIAgent::Auggie => 0x16A34A,
            CLIAgent::Hermes => 0x8B5CF6,
            CLIAgent::Vibe => 0xFA520F,
            CLIAgent::Antigravity => 0x3186FF,
            CLIAgent::Grok => 0x000000,
            CLIAgent::Qwen => 0x6D44E8,
            CLIAgent::OhMyPi => 0xF97316,
            // The blue of the flame in Kimi's brand mark; the glyph itself is
            // black, which Codex and Grok already have covered.
            CLIAgent::Kimi => 0x027AFF,
        }
    }

    pub fn icon_path(self) -> &'static str {
        match self {
            CLIAgent::Claude => "icons/agents/claude.svg",
            CLIAgent::Codex => "icons/agents/codex.svg",
            CLIAgent::Gemini => "icons/agents/gemini.svg",
            CLIAgent::Amp => "icons/agents/amp.svg",
            CLIAgent::OpenCode => "icons/agents/opencode.svg",
            CLIAgent::Copilot => "icons/agents/copilot.svg",
            CLIAgent::Cursor => "icons/agents/cursor.svg",
            CLIAgent::Goose => "icons/agents/goose.svg",
            CLIAgent::Droid => "icons/agents/droid.svg",
            CLIAgent::Grok => "icons/agents/grok.svg",
            CLIAgent::Pi => "icons/agents/pi.svg",
            CLIAgent::OhMyPi => "icons/agents/omp.svg",
            CLIAgent::Qwen => "icons/agents/qwen.svg",
            CLIAgent::Kimi => "icons/agents/kimi.svg",
            CLIAgent::Antigravity => "icons/agents/antigravity.svg",
            CLIAgent::Aider | CLIAgent::Auggie | CLIAgent::Hermes | CLIAgent::Vibe => {
                "icons/bot.svg"
            }
        }
    }

    fn match_token(token: &str) -> Option<CLIAgent> {
        CLIAgent::ALL
            .into_iter()
            .find(|a| a.aliases().contains(&token))
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn detect_from_argv(argv: &[String]) -> Option<CLIAgent> {
        Self::detect_from_argv_with(argv, &HashMap::new())
    }

    pub fn detect_from_argv_with(
        argv: &[String],
        custom: &HashMap<String, String>,
    ) -> Option<CLIAgent> {
        let mut rest = argv
            .iter()
            .map(String::as_str)
            .skip_while(|t| is_env_assignment(t));

        let launcher = rest.next()?;
        let launcher_stem = base_stem(launcher);

        if let Some(agent) = CLIAgent::match_token(launcher_stem) {
            return Some(agent);
        }
        if let Some(agent) = custom
            .get(&launcher_stem.to_ascii_lowercase())
            .and_then(|slug| CLIAgent::from_slug(slug))
        {
            return Some(agent);
        }

        if is_interpreter(launcher_stem) {
            for arg in rest {
                if arg.starts_with('-') {
                    continue;
                }
                for segment in arg.split(['/', '\\']) {
                    if let Some(agent) =
                        CLIAgent::match_token(&base_stem(segment).to_ascii_lowercase())
                    {
                        return Some(agent);
                    }
                }
            }
        }

        None
    }

    /// Identity from an interpreter's *image path*, not from argv.
    ///
    /// ToolHelp often shows only `node.exe`. The install directory still
    /// names the agent (`…\cursor-agent\…\node.exe`, `…\@openai\codex\…`).
    /// A home-folder or project directory named `claude` / `pi` / `amp`
    /// must not count: those aliases are too short to trust as an
    /// arbitrary path segment. Only a hyphenated alias or a scoped npm
    /// package (`@openai/codex`) is distinctive enough.
    pub fn detect_from_image_path_with(
        path: &std::path::Path,
        custom: &HashMap<String, String>,
    ) -> Option<CLIAgent> {
        let components: Vec<String> = path
            .iter()
            .map(|c| c.to_string_lossy().to_ascii_lowercase())
            .collect();

        for (i, component) in components.iter().enumerate() {
            let stem = base_stem(component);
            let prev = i.checked_sub(1).map(|j| components[j].as_str());
            if !image_path_component_is_distinctive(stem, prev) {
                continue;
            }
            if let Some(agent) = Self::match_token(stem) {
                return Some(agent);
            }
            if let Some(agent) = custom.get(stem).and_then(|slug| CLIAgent::from_slug(slug)) {
                return Some(agent);
            }
        }
        None
    }

    pub fn detect_from_command_with(
        command: &str,
        custom: &HashMap<String, String>,
    ) -> Option<CLIAgent> {
        let argv: Vec<String> = command_argv(command)
            .iter()
            .map(|t| t.to_ascii_lowercase())
            .collect();
        Self::detect_from_argv_with(&argv, custom)
    }
}

/// Splits a shell-integration command capture into argv tokens, preserving
/// case so the result can serve as `launch_argv` for flag replay on resume.
/// Keep quoted text in one token: a flag mentioned inside a prompt is data.
/// This only groups quotes, without shell expansion or backslash unescaping;
/// `replay_flags` rejects tokens that cannot safely be replayed.
pub fn command_argv(command: &str) -> Vec<String> {
    let mut argv = Vec::new();
    let mut token = String::new();
    let mut quote = None;
    let mut started = false;
    for ch in command.chars() {
        if let Some(delimiter) = quote {
            if ch == delimiter {
                quote = None;
            } else {
                token.push(ch);
            }
        } else if ch == '"' || ch == '\'' {
            quote = Some(ch);
            started = true;
        } else if ch.is_whitespace() {
            if started {
                argv.push(std::mem::take(&mut token));
                started = false;
            }
        } else {
            token.push(ch);
            started = true;
        }
    }
    if quote.is_some() {
        return Vec::new();
    }
    if started {
        argv.push(token);
    }
    if argv.first().is_some_and(|t| t == "&") {
        argv.remove(0);
    }
    argv
}

fn quoted_history_path(path: &str) -> Option<String> {
    if path.is_empty()
        || path.starts_with('-')
        || path
            .chars()
            .any(|c| c.is_control() || "\"'$`%!".contains(c))
    {
        return None;
    }
    Some(format!("\"{path}\""))
}

fn unambiguous_resume_options(options: &[String]) -> bool {
    let mut args = options.iter().map(String::as_str);
    while let Some(arg) = args.next() {
        let (flag, value) = arg
            .split_once('=')
            .map_or((arg, None), |(f, v)| (f, Some(v)));
        match flag {
            "--yolo"
            | "--dangerously-skip-permissions"
            | "--dangerously-allow-all"
            | "--force"
            | "--plan"
            | "--trust"
            | "--approve-mcps"
                if value.is_none() => {}
            "--model" | "-m" | "--weak-model" | "--editor-model" | "--effort" | "--mode"
            | "--permission-mode" | "--session-dir" | "--cwd" | "--project" | "--agent"
            | "--add-dir" => {
                if value.is_none() && args.next().is_none() {
                    return false;
                }
            }
            _ => return false,
        }
    }
    true
}

fn is_env_assignment(token: &str) -> bool {
    match token.split_once('=') {
        Some((key, _)) => {
            let mut bytes = key.bytes();
            bytes
                .next()
                .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
                && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
        }
        None => false,
    }
}

fn base_stem(token: &str) -> &str {
    let trimmed = token.trim_end_matches(['/', '\\']);
    let name = match trimmed.rfind(['/', '\\']) {
        Some(i) => &trimmed[i + 1..],
        None => trimmed,
    };
    for ext in [
        ".js", ".mjs", ".cjs", ".ts", ".py", ".rb", ".sh", ".exe", ".cmd", ".bat", ".ps1",
    ] {
        if let Some(stem) = name.strip_suffix(ext) {
            return stem;
        }
    }
    name
}

fn image_path_component_is_distinctive(stem: &str, prev: Option<&str>) -> bool {
    stem.contains('-') || prev.is_some_and(|p| p.starts_with('@'))
}

fn is_interpreter(stem: &str) -> bool {
    matches!(
        stem.to_ascii_lowercase().as_str(),
        "node"
            | "nodejs"
            | "bun"
            | "deno"
            | "npx"
            | "pnpm"
            | "yarn"
            | "python"
            | "python3"
            | "ruby"
            | "uv"
            | "uvx"
            | "env"
    )
}

pub const AGENT_EVENT_SENTINEL: &str = "tty7://cli-agent";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentStatus {
    #[default]
    Idle,
    Working,
    Waiting,
    Done,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSessionState {
    #[serde(default = "AgentSessionState::default_status")]
    pub status: AgentStatus,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub launch_argv: Option<Vec<String>>,
    /// A plain new launch with no user input or conversation activity yet.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub unstarted: bool,
    #[serde(default)]
    pub rich: bool,
    #[serde(default)]
    pub cwd: Option<std::path::PathBuf>,
    #[serde(default)]
    pub activity: u64,
}

impl AgentStatus {
    pub fn dot_rgb(self) -> Option<u32> {
        match self {
            AgentStatus::Idle => None,
            AgentStatus::Working => Some(0x3B82F6),
            AgentStatus::Waiting => Some(0xF59E0B),
            AgentStatus::Done => Some(0x22C55E),
        }
    }
}

impl AgentSessionState {
    fn default_status() -> AgentStatus {
        AgentStatus::Idle
    }

    pub fn apply_event(&mut self, ev: &AgentEvent) {
        // SessionStart can allocate an id before any conversation history
        // exists. Keep an untouched launch until actual activity arrives.
        if ev.kind != AgentEventKind::SessionStart {
            self.unstarted = false;
        }
        self.rich = true;
        if let Some(id) = &ev.session_id {
            self.session_id = Some(id.clone());
        }
        if let Some(cwd) = &ev.cwd {
            self.cwd = Some(cwd.clone());
        }
        match ev.kind {
            AgentEventKind::SessionStart => {
                self.status = AgentStatus::Idle;
                self.message = None;
            }
            AgentEventKind::PromptSubmit => {
                self.status = AgentStatus::Working;
                self.message = None;
            }
            AgentEventKind::PermissionRequest | AgentEventKind::QuestionAsked => {
                self.status = AgentStatus::Waiting;
                self.message = ev.message.clone();
            }
            AgentEventKind::Notification => {
                if self.status == AgentStatus::Working {
                    self.status = AgentStatus::Waiting;
                    self.message = ev.message.clone();
                }
            }
            AgentEventKind::ToolComplete => {
                self.activity = self.activity.wrapping_add(1);
                if self.status == AgentStatus::Waiting {
                    self.status = AgentStatus::Working;
                    self.message = None;
                }
            }
            AgentEventKind::Stop => {
                self.status = AgentStatus::Done;
                self.message = ev.message.clone();
            }
            AgentEventKind::SessionEnd => {
                self.status = AgentStatus::Idle;
                self.message = None;
                self.cwd = None;
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentEventKind {
    SessionStart,
    PromptSubmit,
    PermissionRequest,
    QuestionAsked,
    ToolComplete,
    Notification,
    Stop,
    SessionEnd,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentEvent {
    pub agent: Option<CLIAgent>,
    pub kind: AgentEventKind,
    pub session_id: Option<String>,
    pub message: Option<String>,
    pub cwd: Option<std::path::PathBuf>,
    /// What the user typed, on a `PromptSubmit` — already clamped to a label's
    /// worth of text by the hook that sent it, since this rides an OSC payload
    /// the tokenizer abandons rather than truncates past 8 KiB.
    ///
    /// Separate from `message`, which carries what the *agent* said and is
    /// deliberately cleared when a turn starts.
    pub prompt: Option<String>,
}

pub fn parse_agent_event(payload: &[u8]) -> Option<AgentEvent> {
    let rest = payload.strip_prefix(b"777;notify;")?;
    let rest = rest.strip_prefix(AGENT_EVENT_SENTINEL.as_bytes())?;
    let json = rest.strip_prefix(b";")?;

    #[derive(Deserialize)]
    struct Wire {
        #[serde(default)]
        #[allow(dead_code)]
        v: u32,
        #[serde(default)]
        agent: Option<String>,
        event: String,
        #[serde(default)]
        session_id: Option<String>,
        #[serde(default)]
        message: Option<String>,
        #[serde(default)]
        cwd: Option<String>,
        #[serde(default)]
        prompt: Option<String>,
    }

    let w: Wire = serde_json::from_slice(json).ok()?;
    let kind = serde_json::from_value::<AgentEventKind>(serde_json::Value::String(w.event)).ok()?;
    let nonempty = |s: Option<String>| s.filter(|s| !s.trim().is_empty());
    Some(AgentEvent {
        agent: w.agent.as_deref().and_then(CLIAgent::from_slug),
        kind,
        session_id: nonempty(w.session_id),
        message: nonempty(w.message),
        cwd: nonempty(w.cwd).map(std::path::PathBuf::from),
        prompt: nonempty(w.prompt),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn unstarted_launches_exclude_history_selectors_and_prompts() {
        for agent in CLIAgent::ALL {
            let launch = agent.aliases()[0];
            assert_eq!(
                agent.unstarted_command(&command_argv(launch)).as_deref(),
                Some(launch)
            );
            for tail in [
                "resume",
                "--resume",
                "--continue",
                "--last",
                "--session latest",
                "--prompt hello",
                "hello",
                "--config resume=true",
                "--model",
            ] {
                assert!(
                    agent
                        .unstarted_command(&command_argv(&format!("{launch} {tail}")))
                        .is_none(),
                    "{launch} {tail} cannot prove a new, empty launch"
                );
            }
        }
        assert_eq!(
            CLIAgent::Codex
                .unstarted_command(&command_argv("codex --yolo"))
                .as_deref(),
            Some("codex --yolo")
        );
        assert_eq!(
            CLIAgent::Goose
                .unstarted_command(&command_argv("goose session"))
                .as_deref(),
            Some("goose session")
        );
    }

    #[test]
    fn detects_native_binaries() {
        assert_eq!(
            CLIAgent::detect_from_argv(&argv(&["claude"])),
            Some(CLIAgent::Claude)
        );
        assert_eq!(
            CLIAgent::detect_from_argv(&argv(&["/opt/homebrew/bin/codex", "--model", "o3"])),
            Some(CLIAgent::Codex)
        );
        assert_eq!(
            CLIAgent::detect_from_argv(&argv(&["/usr/local/bin/gemini"])),
            Some(CLIAgent::Gemini)
        );
        assert_eq!(
            CLIAgent::detect_from_argv(&argv(&["cursor-agent"])),
            Some(CLIAgent::Cursor)
        );
        assert_eq!(
            CLIAgent::detect_from_argv(&argv(&["claude/"])),
            Some(CLIAgent::Claude)
        );
    }

    #[test]
    fn strips_leading_env_assignments() {
        assert_eq!(
            CLIAgent::detect_from_argv(&argv(&["FOO=1", "BAR=baz", "claude"])),
            Some(CLIAgent::Claude)
        );
    }

    #[test]
    fn an_interpreter_image_path_only_matches_a_distinctive_install_prefix() {
        use std::path::PathBuf;

        // Build paths from components so this Windows-focused detection test
        // still exercises the same component boundaries on Unix CI runners.
        let native_path = |components: &[&str]| components.iter().collect::<PathBuf>();

        assert_eq!(
            CLIAgent::detect_from_image_path_with(
                &native_path(&[
                    "Users",
                    "me",
                    "AppData",
                    "Local",
                    "cursor-agent",
                    "versions",
                    "current",
                    "node.exe",
                ]),
                &HashMap::new(),
            ),
            Some(CLIAgent::Cursor)
        );
        assert_eq!(
            CLIAgent::detect_from_image_path_with(
                &native_path(&[
                    "Users",
                    "me",
                    "AppData",
                    "Roaming",
                    "npm",
                    "node_modules",
                    "@openai",
                    "codex",
                    "vendor",
                    "node.exe",
                ]),
                &HashMap::new(),
            ),
            Some(CLIAgent::Codex),
            "a scoped npm package is an install prefix, not a folder name"
        );
        assert_eq!(
            CLIAgent::detect_from_image_path_with(
                &native_path(&["Users", "claude", "AppData", "Local", "fnm", "node.exe"]),
                &HashMap::new(),
            ),
            None,
            "a home-directory name must not count as the agent"
        );
        assert_eq!(
            CLIAgent::detect_from_image_path_with(
                &native_path(&["Users", "me", "pi", "node.exe"]),
                &HashMap::new(),
            ),
            None
        );
        assert_eq!(
            CLIAgent::detect_from_image_path_with(
                &native_path(&[
                    "Users", "amp", ".nvm", "versions", "node", "v22.0.0", "bin", "node",
                ]),
                &HashMap::new(),
            ),
            None
        );
    }

    #[test]
    fn detects_node_wrapped_claude_by_package_dir() {
        assert_eq!(
            CLIAgent::detect_from_argv(&argv(&[
                "node",
                "/Users/x/.npm/_npx/node_modules/@anthropic-ai/claude-code/cli.js",
            ])),
            Some(CLIAgent::Claude)
        );
    }

    #[test]
    fn detects_npx_package_form() {
        assert_eq!(
            CLIAgent::detect_from_argv(&argv(&["npx", "@anthropic-ai/claude-code"])),
            Some(CLIAgent::Claude)
        );
        assert_eq!(
            CLIAgent::detect_from_argv(&argv(&["npx", "@google/gemini-cli"])),
            Some(CLIAgent::Gemini)
        );
    }

    #[test]
    fn detects_python_wrapped_aider() {
        assert_eq!(
            CLIAgent::detect_from_argv(&argv(&[
                "python3",
                "/usr/lib/python3.12/site-packages/aider/__main__.py",
            ])),
            Some(CLIAgent::Aider)
        );
    }

    #[test]
    fn non_interpreter_does_not_match_on_arguments() {
        assert_eq!(
            CLIAgent::detect_from_argv(&argv(&["cat", "codex.md"])),
            None
        );
        assert_eq!(
            CLIAgent::detect_from_argv(&argv(&["vim", "claude-code/notes.txt"])),
            None
        );
        assert_eq!(CLIAgent::detect_from_argv(&argv(&["less", "aider"])), None);
    }

    #[test]
    fn unrelated_commands_are_none() {
        assert_eq!(CLIAgent::detect_from_argv(&argv(&["zsh"])), None);
        assert_eq!(
            CLIAgent::detect_from_argv(&argv(&["node", "server.js"])),
            None
        );
        assert_eq!(CLIAgent::detect_from_argv(&argv(&[])), None);
    }

    #[test]
    fn every_agent_has_metadata() {
        for a in CLIAgent::ALL {
            assert!(!a.display_name().is_empty());
            assert!(!a.aliases().is_empty());
            assert!(a.accent_rgb() <= 0xFFFFFF);
            assert_eq!(CLIAgent::from_slug(a.slug()), Some(a));
        }
    }

    #[test]
    fn black_branded_avatars_keep_their_brand_field() {
        assert_eq!(CLIAgent::Codex.accent_rgb(), 0x000000);
        assert_eq!(CLIAgent::Grok.accent_rgb(), 0x000000);
    }

    #[test]
    fn antigravity_uses_its_brand_mark() {
        assert_eq!(
            CLIAgent::Antigravity.icon_path(),
            "icons/agents/antigravity.svg"
        );
    }

    #[test]
    fn only_the_unbranded_agents_use_the_fallback_glyph() {
        let fallback: Vec<&str> = CLIAgent::ALL
            .into_iter()
            .filter(|a| a.icon_path() == "icons/bot.svg")
            .map(CLIAgent::slug)
            .collect();
        assert_eq!(fallback, ["aider", "auggie", "hermes", "vibe"]);
        assert!(
            !fallback.contains(&"omp"),
            "Oh My Pi ships its own mark and must not fall back"
        );
        for a in CLIAgent::ALL {
            let path = a.icon_path();
            assert!(
                path == "icons/bot.svg" || path == format!("icons/agents/{}.svg", a.slug()),
                "{} points at an unexpected {path}",
                a.display_name()
            );
        }
    }

    #[test]
    fn detects_newer_agents_by_command() {
        for (cmd, agent) in [
            ("auggie", CLIAgent::Auggie),
            ("agy", CLIAgent::Antigravity),
            ("vibe-acp", CLIAgent::Vibe),
            ("grok", CLIAgent::Grok),
            ("/usr/local/bin/qwen", CLIAgent::Qwen),
            ("pi", CLIAgent::Pi),
            ("hermes", CLIAgent::Hermes),
            ("omp", CLIAgent::OhMyPi),
            ("/opt/homebrew/bin/omp", CLIAgent::OhMyPi),
            ("kimi", CLIAgent::Kimi),
            ("/usr/local/bin/kimi", CLIAgent::Kimi),
        ] {
            assert_eq!(CLIAgent::detect_from_argv(&argv(&[cmd])), Some(agent));
        }
    }

    /// Oh My Pi is a Pi fork, but it is its own binary with its own config
    /// directory and its own flags — one must never be detected as the other.
    #[test]
    fn oh_my_pi_and_pi_stay_distinct() {
        assert_eq!(
            CLIAgent::detect_from_argv(&argv(&["omp"])),
            Some(CLIAgent::OhMyPi)
        );
        assert_eq!(
            CLIAgent::detect_from_argv(&argv(&["pi"])),
            Some(CLIAgent::Pi)
        );
        assert_eq!(CLIAgent::from_slug("omp"), Some(CLIAgent::OhMyPi));
        assert_eq!(CLIAgent::from_slug("oh-my-pi"), None);
        assert_ne!(CLIAgent::Pi.icon_path(), CLIAgent::OhMyPi.icon_path());
    }

    #[test]
    fn custom_rules_map_wrappers_to_agents() {
        let custom: HashMap<String, String> = [("cc".to_string(), "claude".to_string())].into();
        assert_eq!(
            CLIAgent::detect_from_argv_with(&argv(&["/home/x/bin/cc", "-c"]), &custom),
            Some(CLIAgent::Claude)
        );
        let bogus: HashMap<String, String> = [("cc".to_string(), "hal9000".to_string())].into();
        assert_eq!(
            CLIAgent::detect_from_argv_with(&argv(&["cc"]), &bogus),
            None
        );
        assert_eq!(
            CLIAgent::detect_from_argv_with(&argv(&["node", "cc/cli.js"]), &custom),
            None
        );
        let shadow: HashMap<String, String> = [("codex".to_string(), "claude".to_string())].into();
        assert_eq!(
            CLIAgent::detect_from_argv_with(&argv(&["codex"]), &shadow),
            Some(CLIAgent::Codex)
        );
    }

    #[test]
    fn detects_from_typed_command_lines() {
        let none = HashMap::new();
        assert_eq!(
            CLIAgent::detect_from_command_with("claude --resume abc", &none),
            Some(CLIAgent::Claude)
        );
        assert_eq!(
            CLIAgent::detect_from_command_with("claude.exe", &none),
            Some(CLIAgent::Claude)
        );
        assert_eq!(
            CLIAgent::detect_from_command_with(
                r"C:\Users\x\AppData\Roaming\npm\claude.cmd --model opus",
                &none
            ),
            Some(CLIAgent::Claude)
        );
        assert_eq!(
            CLIAgent::detect_from_command_with("CLAUDE", &none),
            Some(CLIAgent::Claude)
        );
        assert_eq!(
            CLIAgent::detect_from_command_with(r#"& "C:\tools\codex.exe""#, &none),
            Some(CLIAgent::Codex)
        );
        assert_eq!(
            CLIAgent::detect_from_command_with(
                r"node C:\x\node_modules\@anthropic-ai\claude-code\cli.js",
                &none
            ),
            Some(CLIAgent::Claude)
        );
        assert_eq!(
            CLIAgent::detect_from_command_with("npx.cmd @google/gemini-cli", &none),
            Some(CLIAgent::Gemini)
        );
        assert_eq!(
            CLIAgent::detect_from_command_with("notepad claude.txt", &none),
            None
        );
        assert_eq!(
            CLIAgent::detect_from_command_with("cat codex.md", &none),
            None
        );
        assert_eq!(CLIAgent::detect_from_command_with("", &none), None);
        let custom: HashMap<String, String> = [("cc".to_string(), "claude".to_string())].into();
        assert_eq!(
            CLIAgent::detect_from_command_with("cc -c", &custom),
            Some(CLIAgent::Claude)
        );
    }

    #[test]
    fn command_argv_preserves_case_for_flag_replay() {
        assert_eq!(
            command_argv("claude --dangerously-skip-permissions"),
            ["claude", "--dangerously-skip-permissions"]
        );
        assert_eq!(
            command_argv("claude --model Opus --resume Abc-123"),
            ["claude", "--model", "Opus", "--resume", "Abc-123"]
        );
        assert_eq!(
            command_argv(r#"& "C:\Tools\claude.exe" --continue"#),
            [r"C:\Tools\claude.exe", "--continue"]
        );
        assert_eq!(command_argv("  "), [""; 0]);
        assert_eq!(
            command_argv(
                r#"& "C:\Program Files\claude.exe" --system-prompt "explain --resume example""#
            ),
            [
                r"C:\Program Files\claude.exe",
                "--system-prompt",
                "explain --resume example"
            ]
        );
        assert!(command_argv("claude --system-prompt 'unfinished").is_empty());
        assert_eq!(
            CLIAgent::Claude
                .resume_command(
                    "abc",
                    Some(&command_argv("claude --dangerously-skip-permissions"))
                )
                .as_deref(),
            Some("claude --dangerously-skip-permissions --resume abc"),
            "a shell-integration command capture must round-trip into a resume \
             command that keeps the launch flags"
        );
    }

    #[test]
    fn parses_sentinel_events() {
        let ev = parse_agent_event(
            br#"777;notify;tty7://cli-agent;{"v":1,"agent":"claude","event":"permission-request","session_id":"abc-123","message":"Claude needs your permission to use Bash"}"#,
        )
        .expect("well-formed sentinel event");
        assert_eq!(ev.agent, Some(CLIAgent::Claude));
        assert_eq!(ev.kind, AgentEventKind::PermissionRequest);
        assert_eq!(ev.session_id.as_deref(), Some("abc-123"));
        assert!(ev.message.as_deref().unwrap().contains("permission"));

        assert_eq!(parse_agent_event(b"777;notify;Build;done"), None);
        assert_eq!(
            parse_agent_event(br#"777;notify;tty7://cli-agent;{"event":"quantum-leap"}"#),
            None
        );
        assert_eq!(
            parse_agent_event(b"777;notify;tty7://cli-agent;{oops"),
            None
        );
    }

    #[test]
    fn session_state_machine_follows_the_turn() {
        let mut s = AgentSessionState::default();
        assert_eq!(s.status, AgentStatus::Idle);

        let ev = |kind, msg: Option<&str>, id: Option<&str>| AgentEvent {
            agent: Some(CLIAgent::Claude),
            kind,
            session_id: id.map(String::from),
            message: msg.map(String::from),
            cwd: None,
            prompt: None,
        };

        s.apply_event(&ev(AgentEventKind::SessionStart, None, Some("sid-1")));
        assert_eq!(s.status, AgentStatus::Idle);
        assert_eq!(s.session_id.as_deref(), Some("sid-1"));
        assert!(s.rich);

        s.apply_event(&ev(AgentEventKind::PromptSubmit, None, None));
        assert_eq!(s.status, AgentStatus::Working);

        s.apply_event(&ev(
            AgentEventKind::Notification,
            Some("Claude needs your permission"),
            None,
        ));
        assert_eq!(s.status, AgentStatus::Waiting);
        assert!(s.message.as_deref().unwrap().contains("permission"));

        s.apply_event(&ev(AgentEventKind::ToolComplete, None, None));
        assert_eq!(s.status, AgentStatus::Working);
        assert_eq!(s.message, None, "the stale permission prompt is cleared");

        s.apply_event(&ev(AgentEventKind::ToolComplete, None, None));
        assert_eq!(s.status, AgentStatus::Working);

        s.apply_event(&ev(AgentEventKind::Stop, None, None));
        assert_eq!(s.status, AgentStatus::Done);

        s.apply_event(&ev(AgentEventKind::ToolComplete, None, None));
        assert_eq!(s.status, AgentStatus::Done);

        s.apply_event(&ev(
            AgentEventKind::Notification,
            Some("Claude is waiting for your input"),
            None,
        ));
        assert_eq!(
            s.status,
            AgentStatus::Done,
            "an idle notification between turns must not fabricate a block"
        );

        s.apply_event(&ev(AgentEventKind::SessionEnd, None, None));
        assert_eq!(s.status, AgentStatus::Idle);
        assert_eq!(s.session_id.as_deref(), Some("sid-1"));
    }

    #[test]
    fn tool_completions_count_even_when_the_status_holds_still() {
        let ev = |kind| AgentEvent {
            agent: Some(CLIAgent::Claude),
            kind,
            session_id: None,
            message: None,
            cwd: None,
            prompt: None,
        };

        let mut s = AgentSessionState::default();
        s.apply_event(&ev(AgentEventKind::PromptSubmit));
        assert_eq!(s.activity, 0, "a turn starting is not tool activity");

        for n in 1..=3 {
            s.apply_event(&ev(AgentEventKind::ToolComplete));
            assert_eq!(s.status, AgentStatus::Working, "the status holds still…");
            assert_eq!(s.activity, n, "…while the counter is what moves");
        }

        s.apply_event(&ev(AgentEventKind::Stop));
        s.apply_event(&ev(AgentEventKind::ToolComplete));
        assert_eq!(
            s.status,
            AgentStatus::Done,
            "and still doesn't resurrect the turn"
        );
        assert_eq!(s.activity, 4);

        s.apply_event(&ev(AgentEventKind::SessionEnd));
        assert_eq!(s.activity, 4);
    }

    #[test]
    fn session_state_tracks_and_releases_the_agent_cwd() {
        use std::path::PathBuf;

        let ev = |kind, cwd: Option<&str>| AgentEvent {
            agent: Some(CLIAgent::Claude),
            kind,
            session_id: None,
            message: None,
            cwd: cwd.map(PathBuf::from),
            prompt: None,
        };

        let mut s = AgentSessionState::default();
        s.apply_event(&ev(AgentEventKind::SessionStart, Some("/repo")));
        assert_eq!(s.cwd.as_deref(), Some(std::path::Path::new("/repo")));

        s.apply_event(&ev(
            AgentEventKind::ToolComplete,
            Some("/repo/.claude/worktrees/fix-x"),
        ));
        assert_eq!(
            s.cwd.as_deref(),
            Some(std::path::Path::new("/repo/.claude/worktrees/fix-x"))
        );

        s.apply_event(&ev(AgentEventKind::Stop, None));
        assert_eq!(
            s.cwd.as_deref(),
            Some(std::path::Path::new("/repo/.claude/worktrees/fix-x"))
        );

        s.apply_event(&ev(AgentEventKind::SessionEnd, None));
        assert_eq!(s.cwd, None, "session end releases the cwd claim");
    }

    #[test]
    fn every_saved_resume_command_can_recover_its_exact_session_id() {
        let id = "0199c3f2-1b0e-7c3a-9f21-6d4b8e2a5c17";
        for agent in CLIAgent::ALL {
            let Some(command) = agent.resume_command(id, None) else {
                continue;
            };
            let argv = command_argv(&command);
            assert_eq!(
                agent.resumed_session_id(&argv),
                Some(id),
                "{agent:?}: a persisted explicit resume must not turn into a bare shell"
            );
            if let Some(fork) = agent.fork_command(id, None) {
                assert_eq!(
                    agent.resumed_session_id(&command_argv(&fork)),
                    None,
                    "{agent:?}: a fork's parent is not the pane's new conversation"
                );
            }
        }
    }

    #[test]
    fn recovered_targets_are_complete_ids_not_names_indexes_or_option_values() {
        let id = "0199c3f2-1b0e-7c3a-9f21-6d4b8e2a5c17";
        for agent in CLIAgent::ALL {
            if agent == CLIAgent::Aider {
                continue;
            }
            for target in ["latest", "last", "123", "my-session"] {
                let command = agent.resume_command(target, None).unwrap();
                assert_eq!(
                    agent.resumed_session_id(&command_argv(&command)),
                    None,
                    "{command}"
                );
            }
            assert!(agent.resume_command("--last", None).is_none());
            if !matches!(agent, CLIAgent::Codex | CLIAgent::Claude) {
                let launch = command_argv(&format!(
                    "{} --unknown-option",
                    agent.resume_command(id, None).unwrap()
                ));
                assert!(agent.resumed_session_id(&launch).is_none());
            }
        }
        for (agent, id) in [
            (CLIAgent::Amp, "T-0199c3f2-1b0e-7c3a-9f21-6d4b8e2a5c17"),
            (CLIAgent::OpenCode, "ses_36c1f475affeXzwMW17xl4nTOJ"),
            (CLIAgent::Goose, "20260911_1"),
        ] {
            let command = agent.resume_command(id, None).unwrap();
            assert_eq!(agent.resumed_session_id(&command_argv(&command)), Some(id));
        }
        for agent in [CLIAgent::Gemini, CLIAgent::Cursor, CLIAgent::Antigravity] {
            let command = agent.resume_command(id, None).unwrap();
            let program = command_argv(&command)[0].clone();
            let misleading = format!("{program} --unknown-option --resume {id}");
            assert_eq!(agent.resumed_session_id(&command_argv(&misleading)), None);
        }
    }

    #[test]
    fn aider_restores_the_selected_history_and_drops_the_previous_file() {
        let launch = command_argv(
            "aider --model model-name --chat-history-file old.md --no-restore-chat-history",
        );
        let file = r"C:\Work\chat history.md";
        let command = CLIAgent::Aider.resume_command(file, Some(&launch)).unwrap();
        assert_eq!(
            command,
            format!(
                "aider --model model-name --chat-history-file \"{file}\" --restore-chat-history"
            )
        );
        assert_eq!(
            CLIAgent::Aider.resumed_session_id(&command_argv(&command)),
            Some(file)
        );
        assert!(
            CLIAgent::Aider
                .resumed_session_id(&command_argv("aider --model model-name"))
                .is_none()
        );
    }

    #[test]
    fn resume_ids_are_explicit_and_never_fork_parents() {
        let id = "0199c3f2-1b0e-7c3a-9f21-6d4b8e2a5c17";
        for (agent, prefix, suffix) in [
            (CLIAgent::Codex, "codex resume", "--yolo"),
            (CLIAgent::Codex, r"C:\Tools\codex.exe resume", "--yolo"),
            (
                CLIAgent::Codex,
                "node /opt/@openai/codex/bin/codex.js resume",
                "--yolo",
            ),
            (
                CLIAgent::Claude,
                "claude --dangerously-skip-permissions --resume",
                "",
            ),
            (CLIAgent::Claude, "claude -r", "--model Opus"),
            (
                CLIAgent::Claude,
                "claude --model Opus --permission-mode bypassPermissions --resume",
                "",
            ),
            (CLIAgent::Claude, "env claude --resume", ""),
        ] {
            let captured = command_argv(&format!("{prefix} {id} {suffix}"));
            assert_eq!(
                agent.resumed_session_id(&captured),
                Some(id),
                "{captured:?}"
            );
        }
        assert_eq!(
            CLIAgent::Claude.resumed_session_id(&argv(&["claude", &format!("--resume={id}")])),
            Some(id)
        );
        for (agent, command) in [
            (CLIAgent::Codex, format!("codex fork {id} --yolo")),
            (CLIAgent::Codex, "codex resume --last".into()),
            (CLIAgent::Codex, format!("codex resume {id} --last")),
            (CLIAgent::Codex, format!("codex -- {id}")),
            (CLIAgent::Codex, "codex resume my-session".into()),
            (CLIAgent::Codex, "codex resume $(whoami)".into()),
            (CLIAgent::Codex, format!("echo codex resume {id}")),
            (CLIAgent::Codex, format!("codex resume {id} && codex")),
            (
                CLIAgent::Claude,
                format!("claude --resume {id} --fork-session"),
            ),
            (
                CLIAgent::Claude,
                format!("claude --resume {id} --fork-session=true"),
            ),
            (CLIAgent::Claude, format!("claude --continue --resume {id}")),
            (CLIAgent::Claude, format!("claude -- --resume {id}")),
            (
                CLIAgent::Claude,
                format!("claude --resume {id} --resume {id}"),
            ),
            (CLIAgent::Claude, "claude --resume my-session".into()),
        ] {
            assert_eq!(
                agent.resumed_session_id(&command_argv(&command)),
                None,
                "{command}"
            );
        }
    }

    #[test]
    fn claude_resume_selectors_are_not_option_values() {
        let id = "0199c3f2-1b0e-7c3a-9f21-6d4b8e2a5c17";
        for option in ["--append-system-prompt", "--system-prompt", "--model"] {
            let captured = argv(&["claude", option, &format!("--resume={id}")]);
            assert_eq!(
                CLIAgent::Claude.resumed_session_id(&captured),
                None,
                "{captured:?}"
            );
        }
        for prompt in [
            format!("--resume {id}"),
            format!("plain text --resume {id}"),
            format!("--resume={id} --resume {id}"),
        ] {
            let command = format!("claude --system-prompt '{prompt}'");
            assert_eq!(
                CLIAgent::Claude.resumed_session_id(&command_argv(&command)),
                None,
                "{command}"
            );
        }
        assert_eq!(
            CLIAgent::Claude.resumed_session_id(&argv(&[
                "claude",
                "--unknown-option",
                &format!("--resume={id}"),
            ])),
            None,
            "an option with unknown arity cannot establish a session id"
        );
        for value in [format!("--resume={id}"), "--fork-session".into()] {
            assert_eq!(
                CLIAgent::Claude.resumed_session_id(&argv(&[
                    "claude",
                    "--append-system-prompt",
                    &value,
                    "--resume",
                    id,
                ])),
                Some(id),
                "only the real selector controls this resume"
            );
        }
    }

    #[test]
    fn known_resume_ids_survive_executable_wrappers_but_not_a_new_session() {
        for (agent, id, command) in [
            (
                CLIAgent::Cursor,
                "cursor-123",
                "cursor-agent --resume cursor-123",
            ),
            (
                CLIAgent::Cursor,
                "cursor-123",
                "node /opt/cursor-agent/index.js --resume cursor-123",
            ),
            (
                CLIAgent::Antigravity,
                "agy-123",
                "C:/Tools/agy.exe --conversation agy-123",
            ),
            (
                CLIAgent::Grok,
                "grok-123",
                "grok --model grok-code-fast-1 --resume grok-123",
            ),
            (
                CLIAgent::OpenCode,
                "ses_123abc",
                "opencode --session ses_123abc",
            ),
            (CLIAgent::Amp, "T-123abc", "amp threads continue T-123abc"),
            (
                CLIAgent::Goose,
                "20260911_1",
                "goose session --resume --session-id 20260911_1",
            ),
            (CLIAgent::Pi, "pi-123", "pi --session pi-123"),
        ] {
            let captured = command_argv(command);
            assert!(agent.resumes_session(id, &captured), "{command}");
            assert!(
                !agent.resumes_session("different-session", &captured),
                "{command}"
            );
            assert_eq!(
                agent.resumed_session_id(&captured),
                (agent == CLIAgent::Goose).then_some(id),
                "only a complete native id may be recovered from this command"
            );
        }
        for (agent, command) in [
            (CLIAgent::Cursor, "cursor-agent --continue"),
            (CLIAgent::Cursor, "cursor-agent"),
            (CLIAgent::Antigravity, "agy --continue"),
            (CLIAgent::Grok, "grok --resume known --fork-session"),
            (CLIAgent::Amp, "amp threads fork known"),
            (CLIAgent::OpenCode, "opencode --session known --fork"),
            (CLIAgent::Aider, "aider"),
        ] {
            assert!(
                !agent.resumes_session("known", &command_argv(command)),
                "{command}"
            );
        }
    }

    #[test]
    fn resume_commands_are_shell_safe() {
        assert_eq!(
            CLIAgent::Claude.resume_command("abc-123", None).as_deref(),
            Some("claude --resume abc-123")
        );
        assert_eq!(
            CLIAgent::Codex.resume_command("th_read.9", None).as_deref(),
            Some("codex resume th_read.9")
        );
        assert_eq!(
            CLIAgent::Pi
                .resume_command("0199c3f2-1b0e-7c3a-9f21-6d4b8e2a5c17", None)
                .as_deref(),
            Some("pi --session 0199c3f2-1b0e-7c3a-9f21-6d4b8e2a5c17")
        );
        assert_eq!(
            CLIAgent::Kimi.resume_command("abc-123", None).as_deref(),
            Some("kimi --session abc-123")
        );
        assert_eq!(
            CLIAgent::Aider
                .resume_command("chat history.md", None)
                .as_deref(),
            Some("aider --chat-history-file \"chat history.md\" --restore-chat-history")
        );
        for path in [
            "",
            "$(boom)",
            "%TEMP%/history.md",
            "history\nexit",
            "--model",
        ] {
            assert!(CLIAgent::Aider.resume_command(path, None).is_none());
        }
        assert_eq!(CLIAgent::Claude.resume_command("abc; rm -rf /", None), None);
        assert_eq!(CLIAgent::Claude.resume_command("$(boom)", None), None);
        assert_eq!(CLIAgent::Claude.resume_command("", None), None);
    }

    #[test]
    fn resume_carries_launch_flags() {
        let argv = |parts: &[&str]| parts.iter().map(|s| s.to_string()).collect::<Vec<_>>();

        assert_eq!(
            CLIAgent::Claude
                .resume_command(
                    "abc-123",
                    Some(&argv(&["claude", "--dangerously-skip-permissions"]))
                )
                .as_deref(),
            Some("claude --dangerously-skip-permissions --resume abc-123")
        );
        assert_eq!(
            CLIAgent::Claude
                .resume_command("abc", Some(&argv(&["claude", "--model", "opus"])))
                .as_deref(),
            Some("claude --model opus --resume abc")
        );
        assert_eq!(
            CLIAgent::Kimi
                .resume_command(
                    "abc-123",
                    Some(&argv(&["kimi", "--session", "old", "--yolo"]))
                )
                .as_deref(),
            Some("kimi --yolo --session abc-123"),
            "a stale --session flag comes off before the new one goes on"
        );
        assert_eq!(
            CLIAgent::Kimi
                .resume_command("abc-123", Some(&argv(&["kimi", "--session=old", "--yolo"])))
                .as_deref(),
            Some("kimi --yolo --session abc-123"),
            "and so does the one-token spelling of it"
        );
        assert_eq!(
            CLIAgent::Kimi
                .resume_command(
                    "abc-123",
                    Some(&argv(&["kimi", "--resume", "--model", "kimi-k2"]))
                )
                .as_deref(),
            Some("kimi --model kimi-k2 --session abc-123"),
            "`--session` takes an optional id, so a bare one must not eat the flag after it"
        );
        assert_eq!(
            CLIAgent::Kimi
                .resume_command(
                    "abc-123",
                    Some(&argv(&["kimi", "--continue", "--model", "kimi-k2"]))
                )
                .as_deref(),
            Some("kimi --model kimi-k2 --session abc-123"),
            "`--continue` is mutually exclusive with `--session` and takes no value"
        );
        assert_eq!(
            CLIAgent::Kimi
                .resume_command(
                    "abc-123",
                    Some(&argv(&["kimi", "--agent", "reviewer", "--yolo"]))
                )
                .as_deref(),
            Some("kimi --yolo --session abc-123"),
            "Kimi rejects `--agent` next to `--session`, and resume rebinds the agent itself"
        );
        assert_eq!(
            CLIAgent::Claude
                .resume_command(
                    "abc",
                    Some(&argv(&[
                        "node",
                        "/x/node_modules/@anthropic-ai/claude-code/cli.js",
                        "--dangerously-skip-permissions",
                    ]))
                )
                .as_deref(),
            Some("claude --dangerously-skip-permissions --resume abc")
        );
        assert_eq!(
            CLIAgent::Claude
                .resume_command(
                    "new-id",
                    Some(&argv(&["claude", "--resume", "old-id", "--model", "opus"]))
                )
                .as_deref(),
            Some("claude --model opus --resume new-id")
        );
        assert_eq!(
            CLIAgent::Codex
                .resume_command("id-1", Some(&argv(&["codex", "--yolo"])))
                .as_deref(),
            Some("codex resume id-1 --yolo")
        );
        assert_eq!(
            CLIAgent::Codex
                .resume_command("id-2", Some(&argv(&["codex", "resume", "id-1", "--yolo"])))
                .as_deref(),
            Some("codex resume id-2 --yolo")
        );
        assert_eq!(
            CLIAgent::Claude
                .resume_command(
                    "abc",
                    Some(&argv(&["claude", "--allowedTools", "Bash(git:*)"]))
                )
                .as_deref(),
            Some("claude --resume abc")
        );
        assert_eq!(
            CLIAgent::Claude
                .resume_command("abc", Some(&argv(&["claude", "fix-the-bug"])))
                .as_deref(),
            Some("claude --resume abc")
        );
        assert_eq!(
            CLIAgent::Claude
                .resume_command(
                    "abc",
                    Some(&argv(&[
                        "CLAUDE_CONFIG_DIR=/opt/claude",
                        "claude",
                        "--dangerously-skip-permissions",
                    ]))
                )
                .as_deref(),
            Some("claude --dangerously-skip-permissions --resume abc")
        );
        assert_eq!(
            CLIAgent::Claude
                .resume_command(
                    "abc",
                    Some(&argv(&["claude", "--model", "opus", "review", "this"]))
                )
                .as_deref(),
            Some("claude --resume abc")
        );
        assert_eq!(
            CLIAgent::Codex
                .resume_command(
                    "id-3",
                    Some(&argv(&["codex", "resume", "--last", "--yolo"]))
                )
                .as_deref(),
            Some("codex resume id-3 --yolo")
        );
        assert_eq!(
            CLIAgent::Pi
                .resume_command("id-a", Some(&argv(&["pi", "--model", "opus"])))
                .as_deref(),
            Some("pi --model opus --session id-a")
        );
        assert_eq!(
            CLIAgent::Pi
                .resume_command(
                    "id-b",
                    Some(&argv(&[
                        "pi",
                        "--session",
                        "old-id",
                        "--fork",
                        "old",
                        "-c",
                        "--model",
                        "opus"
                    ]))
                )
                .as_deref(),
            Some("pi --model opus --session id-b")
        );
        assert_eq!(
            CLIAgent::Pi.resume_command(
                "id-x",
                Some(&argv(&["pi", "--no-session", "--model", "opus"]))
            ),
            None
        );
        assert_eq!(
            CLIAgent::Pi
                .resume_command(
                    "id-c",
                    Some(&argv(&[
                        "pi",
                        "--session-dir",
                        "/w/.sessions",
                        "--fork",
                        "old"
                    ]))
                )
                .as_deref(),
            Some("pi --session-dir /w/.sessions --session id-c")
        );
        assert_eq!(
            CLIAgent::Claude
                .resume_command(
                    "abc",
                    Some(&argv(&["cc", "--dangerously-skip-permissions"]))
                )
                .as_deref(),
            Some("claude --resume abc")
        );
        assert_eq!(
            CLIAgent::Amp
                .resume_command("t-1", Some(&argv(&["amp", "--dangerously-allow-all"])))
                .as_deref(),
            Some("amp threads continue t-1 --dangerously-allow-all")
        );
        assert_eq!(
            CLIAgent::Amp
                .resume_command("t-2", Some(&argv(&["amp", "threads", "continue", "t-1"])))
                .as_deref(),
            Some("amp threads continue t-2")
        );
        assert_eq!(
            CLIAgent::Copilot
                .resume_command(
                    "s-9",
                    Some(&argv(&["copilot", "--resume", "s-1", "--allow-all-tools"]))
                )
                .as_deref(),
            Some("copilot --allow-all-tools --resume s-9")
        );
        assert_eq!(
            CLIAgent::Copilot.resume_command("s-9", None).as_deref(),
            Some("copilot --resume s-9")
        );
        assert_eq!(
            CLIAgent::Grok
                .resume_command("g-2", Some(&argv(&["grok", "--model", "grok-code"])))
                .as_deref(),
            Some("grok --model grok-code --resume g-2")
        );
        assert_eq!(
            CLIAgent::Grok
                .resume_command(
                    "g-2",
                    Some(&argv(&["grok", "--resume", "g-1", "--fork-session"]))
                )
                .as_deref(),
            Some("grok --resume g-2")
        );
        assert_eq!(
            CLIAgent::Grok
                .resume_command(
                    "g-3",
                    Some(&argv(&["grok", "-w", "--worktree-ref", "main", "--yolo"]))
                )
                .as_deref(),
            Some("grok --yolo --resume g-3")
        );
    }

    #[test]
    fn oh_my_pi_resume_and_fork_use_its_own_flags() {
        let argv = |parts: &[&str]| parts.iter().map(|s| s.to_string()).collect::<Vec<_>>();

        assert_eq!(
            CLIAgent::OhMyPi.resume_command("s-1", None).as_deref(),
            Some("omp --resume s-1")
        );
        assert_eq!(
            CLIAgent::OhMyPi.fork_command("s-1", None).as_deref(),
            Some("omp --fork s-1")
        );
        assert_eq!(
            CLIAgent::OhMyPi
                .resume_command(
                    "s-2",
                    Some(&argv(&["omp", "--session", "s-1", "--model", "opus"]))
                )
                .as_deref(),
            Some("omp --model opus --resume s-2"),
            "--session is a third spelling of --resume and sheds with it"
        );
        assert_eq!(
            CLIAgent::OhMyPi
                .fork_command(
                    "s-2",
                    Some(&argv(&["omp", "--fork", "s-1", "-c", "--yolo"]))
                )
                .as_deref(),
            Some("omp --yolo --fork s-2")
        );
        assert_eq!(
            CLIAgent::OhMyPi
                .resume_command(
                    "s-3",
                    Some(&argv(&["omp", "--session-dir", "/w/.sessions"]))
                )
                .as_deref(),
            Some("omp --session-dir /w/.sessions --resume s-3"),
            "--session-dir is a different flag and rides along"
        );

        // `--no-session` means the run never persisted one, so there is
        // nothing to resume from and Oh My Pi rejects `--fork` outright.
        for id in ["s-4"] {
            assert_eq!(
                CLIAgent::OhMyPi.resume_command(id, Some(&argv(&["omp", "--no-session"]))),
                None
            );
            assert_eq!(
                CLIAgent::OhMyPi.fork_command(id, Some(&argv(&["omp", "--no-session"]))),
                None
            );
        }
    }

    #[test]
    fn fork_commands_cover_exactly_the_agents_with_a_verified_fork() {
        assert_eq!(
            CLIAgent::Codex.fork_command("abc-123", None).as_deref(),
            Some("codex fork abc-123")
        );
        assert_eq!(
            CLIAgent::Claude.fork_command("abc-123", None).as_deref(),
            Some("claude --resume abc-123 --fork-session")
        );
        assert_eq!(
            CLIAgent::Grok.fork_command("g-1", None).as_deref(),
            Some("grok --resume g-1 --fork-session")
        );
        assert_eq!(
            CLIAgent::OpenCode.fork_command("s-1", None).as_deref(),
            Some("opencode --session s-1 --fork")
        );
        assert_eq!(
            CLIAgent::Droid.fork_command("session-abc", None).as_deref(),
            Some("droid --fork session-abc")
        );
        assert_eq!(
            CLIAgent::Qwen.fork_command("q-1", None).as_deref(),
            Some("qwen --resume q-1 --fork-session")
        );
        assert_eq!(
            CLIAgent::Goose.fork_command("20260213_9", None).as_deref(),
            Some("goose session --resume --fork --session-id 20260213_9")
        );
        // Undocumented in `amp threads --help`, but `amp threads fork --help`
        // prints its own usage, so the subcommand is real.
        assert_eq!(
            CLIAgent::Amp.fork_command("T-abc", None).as_deref(),
            Some("amp threads fork T-abc")
        );

        // Cursor and Antigravity fork only from inside a running TUI (`/fork`),
        // which is not something a launch command line can reach.
        for agent in [
            CLIAgent::Gemini,
            CLIAgent::Copilot,
            CLIAgent::Cursor,
            CLIAgent::Aider,
            CLIAgent::Auggie,
            CLIAgent::Hermes,
            CLIAgent::Vibe,
            CLIAgent::Antigravity,
        ] {
            assert_eq!(
                agent.fork_command("abc", None),
                None,
                "{} must not claim a fork command",
                agent.slug()
            );
        }

        for agent in CLIAgent::ALL {
            assert_eq!(
                agent.fork_label().is_some(),
                agent.fork_command("abc", None).is_some(),
                "{}: fork_label and fork_command disagree",
                agent.slug()
            );
        }
    }

    #[test]
    fn fork_commands_are_shell_safe() {
        for id in ["abc; rm -rf /", "$(boom)", "", "a b"] {
            assert_eq!(
                CLIAgent::Codex.fork_command(id, None),
                None,
                "codex accepted a non-token id: {id:?}"
            );
            assert_eq!(CLIAgent::Claude.fork_command(id, None), None);
        }
    }

    #[test]
    fn fork_carries_launch_flags_and_sheds_stale_session_targeting() {
        let argv = |parts: &[&str]| parts.iter().map(|s| s.to_string()).collect::<Vec<_>>();

        assert_eq!(
            CLIAgent::Codex
                .fork_command("id-1", Some(&argv(&["codex", "--yolo"])))
                .as_deref(),
            Some("codex fork id-1 --yolo")
        );
        assert_eq!(
            CLIAgent::Claude
                .fork_command(
                    "abc",
                    Some(&argv(&["claude", "--dangerously-skip-permissions"]))
                )
                .as_deref(),
            Some("claude --dangerously-skip-permissions --resume abc --fork-session")
        );

        assert_eq!(
            CLIAgent::Codex
                .fork_command("id-2", Some(&argv(&["codex", "fork", "id-1", "--yolo"])))
                .as_deref(),
            Some("codex fork id-2 --yolo")
        );
        assert_eq!(
            CLIAgent::Claude
                .fork_command(
                    "new",
                    Some(&argv(&["claude", "--resume", "old", "--fork-session"]))
                )
                .as_deref(),
            Some("claude --resume new --fork-session")
        );
        assert_eq!(
            CLIAgent::Grok
                .fork_command(
                    "g-2",
                    Some(&argv(&["grok", "--resume", "g-1", "--fork-session"]))
                )
                .as_deref(),
            Some("grok --resume g-2 --fork-session")
        );
        assert_eq!(
            CLIAgent::OpenCode
                .fork_command(
                    "s-2",
                    Some(&argv(&["opencode", "--session", "s-1", "--fork"]))
                )
                .as_deref(),
            Some("opencode --session s-2 --fork")
        );

        assert_eq!(
            CLIAgent::Codex
                .resume_command("id-2", Some(&argv(&["codex", "fork", "id-1", "--yolo"])))
                .as_deref(),
            Some("codex resume id-2 --yolo")
        );
        assert_eq!(
            CLIAgent::Claude
                .resume_command(
                    "new",
                    Some(&argv(&["claude", "--resume", "old", "--fork-session"]))
                )
                .as_deref(),
            Some("claude --resume new")
        );
        assert_eq!(
            CLIAgent::OpenCode
                .resume_command(
                    "s-2",
                    Some(&argv(&["opencode", "--session", "s-1", "--fork"]))
                )
                .as_deref(),
            Some("opencode --session s-2")
        );
    }

    #[test]
    fn newly_wired_agents_resume_the_way_their_own_cli_spells_it() {
        let argv = |parts: &[&str]| parts.iter().map(|s| s.to_string()).collect::<Vec<_>>();

        for (agent, id, want) in [
            (CLIAgent::Droid, "session-abc", "droid --resume session-abc"),
            (CLIAgent::Qwen, "q-1", "qwen --resume q-1"),
            (CLIAgent::Auggie, "a-1", "auggie --resume a-1"),
            (
                CLIAgent::Goose,
                "20260213_9",
                "goose session --resume --session-id 20260213_9",
            ),
            (
                CLIAgent::Hermes,
                "20260812_213234_5de948",
                "hermes chat --resume 20260812_213234_5de948",
            ),
            (CLIAgent::Vibe, "v-1", "vibe --resume v-1"),
            (
                CLIAgent::Antigravity,
                "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
                "agy --conversation a1b2c3d4-e5f6-7890-abcd-ef1234567890",
            ),
        ] {
            assert_eq!(
                agent.resume_command(id, None).as_deref(),
                Some(want),
                "{} resumes with the wrong command",
                agent.slug()
            );
        }

        // A subcommand-addressed session leaves nothing in `stale` to strip, so
        // the prefix has to be dropped structurally — otherwise the launch flags
        // go down with it.
        assert_eq!(
            CLIAgent::Amp
                .resume_command(
                    "T-2",
                    Some(&argv(&[
                        "amp",
                        "threads",
                        "continue",
                        "T-1",
                        "--dangerously-allow-all",
                    ]))
                )
                .as_deref(),
            Some("amp threads continue T-2 --dangerously-allow-all")
        );
        assert_eq!(
            CLIAgent::Goose
                .resume_command(
                    "20260213_9",
                    Some(&argv(&["goose", "session", "--resume", "--name", "old"]))
                )
                .as_deref(),
            Some("goose session --resume --session-id 20260213_9")
        );
        assert_eq!(
            CLIAgent::Auggie
                .resume_command(
                    "a-2",
                    Some(&argv(&["auggie", "session", "resume", "a-1", "--verbose"]))
                )
                .as_deref(),
            Some("auggie --verbose --resume a-2")
        );
        assert_eq!(
            CLIAgent::Droid
                .resume_command(
                    "s-2",
                    Some(&argv(&["droid", "--fork", "s-1", "--auto", "low"]))
                )
                .as_deref(),
            Some("droid --auto low --resume s-2")
        );

        // `--id` is an alias of `--session-id` and `-n` of `--name`, and the
        // three share one exclusive clap group — any of them surviving next to
        // the `--session-id` the command appends would fail to parse.
        assert_eq!(
            CLIAgent::Goose
                .resume_command(
                    "20260213_9",
                    Some(&argv(&["goose", "s", "--resume", "--id", "20260101_1"]))
                )
                .as_deref(),
            Some("goose session --resume --session-id 20260213_9")
        );
        assert_eq!(
            CLIAgent::Goose
                .resume_command(
                    "20260213_9",
                    Some(&argv(&["goose", "session", "-r", "-n", "old"]))
                )
                .as_deref(),
            Some("goose session --resume --session-id 20260213_9")
        );
        // Qwen rejects `--session-id` alongside `--resume`; Vibe spells
        // `--continue` as `-c` too.
        assert_eq!(
            CLIAgent::Qwen
                .resume_command("q-2", Some(&argv(&["qwen", "--session-id", "old"])))
                .as_deref(),
            Some("qwen --resume q-2")
        );
        assert_eq!(
            CLIAgent::Vibe
                .resume_command("v-2", Some(&argv(&["vibe", "-c"])))
                .as_deref(),
            Some("vibe --resume v-2")
        );

        // Nothing was persisted, so there is nothing to resume or fork.
        for id in ["a-1"] {
            assert_eq!(
                CLIAgent::Auggie
                    .resume_command(id, Some(&argv(&["auggie", "--dont-save-session"]))),
                None
            );
        }
        // "If false, chat history is not saved and --continue/--resume will
        // not work" — so neither resume nor fork is offered.
        let no_recording = argv(&["qwen", "--no-chat-recording"]);
        assert_eq!(
            CLIAgent::Qwen.resume_command("q-1", Some(&no_recording)),
            None
        );
        assert_eq!(
            CLIAgent::Qwen.fork_command("q-1", Some(&no_recording)),
            None
        );
    }

    /// `python3 -m antigravity` opens an xkcd comic. It is the standard way to
    /// trigger Python's easter egg, and the interpreter branch used to read that
    /// module name as an agent.
    #[test]
    fn the_python_easter_egg_is_not_a_coding_agent() {
        assert_eq!(
            CLIAgent::detect_from_argv(&argv(&["python3", "-m", "antigravity"])),
            None
        );
        assert_eq!(
            CLIAgent::detect_from_argv(&argv(&["agy"])),
            Some(CLIAgent::Antigravity)
        );
    }

    #[test]
    fn status_metadata_is_consistent() {
        assert_eq!(AgentStatus::Idle.dot_rgb(), None);
        for st in [
            AgentStatus::Working,
            AgentStatus::Waiting,
            AgentStatus::Done,
        ] {
            assert!(st.dot_rgb().is_some());
        }
        assert_eq!(
            serde_json::to_string(&AgentStatus::Waiting).unwrap(),
            "\"waiting\""
        );
    }
}

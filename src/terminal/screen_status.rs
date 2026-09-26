//! An agent's status read off its screen, the way herdr reads it.
//!
//! The hooks only report what an agent chooses to announce, and some agents
//! announce nothing. Antigravity and Cursor have no hooks tty7 can install,
//! and Codex's hooks say nothing about a permission prompt. Herdr reads all of
//! them from the screen instead: each agent has a manifest of rules, a rule is
//! a set of text matches over one region of the live bottom of the pane, and
//! the highest-priority rule that matches names the state.
//!
//! The manifests in `assets/agent-detection/` are herdr's own, copied without
//! changes (Apache-2.0; see the README there). The matcher and region
//! functions below follow herdr's `src/detect/manifest.rs`, because a manifest
//! means only what its engine makes it mean.

use std::sync::OnceLock;

use alacritty_terminal::event::EventListener;
use alacritty_terminal::term::Term;
use regex::Regex;
use serde::Deserialize;

use crate::core::cli_agent::{AgentSessionState, AgentStatus, CLIAgent};

/// What one look at the screen says.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ScreenState {
    Idle,
    Working,
    Blocked,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Reading {
    pub(crate) state: ScreenState,
    /// Whether a rule matched. Without one this is herdr's fallback (idle for
    /// a known agent that shows nothing it recognises), which is a default and
    /// not evidence.
    pub(crate) matched: bool,
    /// The idle prompt itself is on screen, not just the absence of work.
    pub(crate) visible_idle: bool,
    /// The matched rule marks a screen that says nothing about the state (a
    /// transcript viewer, say), and the previous reading should stand.
    pub(crate) skip: bool,
}

pub(crate) struct Input<'a> {
    pub(crate) screen: &'a str,
    pub(crate) osc_title: &'a str,
    pub(crate) osc_progress: &'a str,
}

/// Classify a screen for `agent`. `None` when herdr has no manifest for it.
pub(crate) fn read(agent: CLIAgent, input: &Input) -> Option<Reading> {
    let manifest = compiled(agent)?;
    let mut best: Option<&CompiledRule> = None;
    for rule in &manifest.rules {
        if !rule.gate.matches(region(input, &rule.region)) {
            continue;
        }
        // Earlier rules win ties, as in herdr.
        if best.is_none_or(|b| rule.priority > b.priority) {
            best = Some(rule);
        }
    }
    Some(match best {
        Some(rule) => Reading {
            state: rule.state,
            matched: true,
            visible_idle: rule.visible_idle && rule.state == ScreenState::Idle,
            skip: rule.skip_state_update,
        },
        None => Reading {
            // Codex's screen is too ambiguous to call idle by default.
            state: match agent {
                CLIAgent::Codex => ScreenState::Unknown,
                _ => ScreenState::Idle,
            },
            matched: false,
            visible_idle: false,
            skip: false,
        },
    })
}

fn manifest_source(agent: CLIAgent) -> Option<&'static str> {
    Some(match agent {
        CLIAgent::Claude => include_str!("../../assets/agent-detection/claude.toml"),
        CLIAgent::Codex => include_str!("../../assets/agent-detection/codex.toml"),
        CLIAgent::Gemini => include_str!("../../assets/agent-detection/gemini.toml"),
        CLIAgent::Amp => include_str!("../../assets/agent-detection/amp.toml"),
        CLIAgent::OpenCode => include_str!("../../assets/agent-detection/opencode.toml"),
        CLIAgent::Copilot => include_str!("../../assets/agent-detection/copilot.toml"),
        CLIAgent::Cursor => include_str!("../../assets/agent-detection/cursor.toml"),
        CLIAgent::Droid => include_str!("../../assets/agent-detection/droid.toml"),
        CLIAgent::Pi => include_str!("../../assets/agent-detection/pi.toml"),
        CLIAgent::Hermes => include_str!("../../assets/agent-detection/hermes.toml"),
        CLIAgent::Antigravity => include_str!("../../assets/agent-detection/antigravity.toml"),
        CLIAgent::Grok => include_str!("../../assets/agent-detection/grok.toml"),
        CLIAgent::Qwen => include_str!("../../assets/agent-detection/qwen.toml"),
        CLIAgent::Kimi => include_str!("../../assets/agent-detection/kimi.toml"),
        CLIAgent::Aider
        | CLIAgent::Goose
        | CLIAgent::Auggie
        | CLIAgent::Vibe
        | CLIAgent::OhMyPi => return None,
    })
}

/// The agents whose hooks herdr trusts over the screen once they report,
/// because they cover the whole turn: permission answers and interrupts
/// included. Every other agent's hooks can miss an Esc or an approval, so the
/// screen gets the last word whenever it has evidence.
fn hooks_are_authority(agent: CLIAgent) -> bool {
    matches!(
        agent,
        CLIAgent::Pi | CLIAgent::OhMyPi | CLIAgent::Kimi | CLIAgent::OpenCode
    )
}

/// The status a pane shows, given what the hooks reported and what the screen
/// says (`status`, and whether a rule backed it).
pub(crate) fn merge(
    agent: CLIAgent,
    hook: Option<&AgentSessionState>,
    screen: Option<(AgentStatus, bool)>,
) -> Option<AgentStatus> {
    let hooked = hook.filter(|s| s.rich).map(|s| s.status);
    if hooked.is_some() && hooks_are_authority(agent) {
        return hooked;
    }
    match screen {
        // A turn short enough to start and end between two looks at the
        // screen still ended, and the hooks saw it.
        Some((AgentStatus::Idle, _)) if hooked == Some(AgentStatus::Done) => hooked,
        Some((status, true)) => Some(status),
        Some((status, false)) => hooked.or(Some(status)),
        None => hook.map(|s| s.status),
    }
}

/// How many polls in a row a drop from working to plain idle has to repeat
/// before it counts. An agent's screen goes quiet between steps of one turn;
/// herdr holds the same drop for about 700 ms.
const IDLE_CONFIRMATIONS: u8 = 2;

/// Screen readings for one pane, turned into a turn's status: a reading of
/// idle after work becomes `Done`, which is what the hooks would have said.
#[derive(Default)]
pub(crate) struct Tracker {
    agent: Option<CLIAgent>,
    /// What the last reading looked at, so an unchanged screen is not read
    /// again.
    seen: Option<(u64, String)>,
    status: Option<AgentStatus>,
    matched: bool,
    held_idle: u8,
}

impl Tracker {
    /// Take another look at the screen if it could have changed since the last
    /// one. `seq` moves whenever output arrives; `screen` renders the snapshot
    /// and is only called when a reading is due.
    pub(crate) fn observe(
        &mut self,
        agent: Option<CLIAgent>,
        seq: u64,
        title: &str,
        screen: impl FnOnce() -> String,
    ) {
        if agent != self.agent {
            *self = Tracker {
                agent,
                ..Tracker::default()
            };
        }
        let Some(agent) = agent.filter(|a| manifest_source(*a).is_some()) else {
            return;
        };
        let unchanged = self
            .seen
            .as_ref()
            .is_some_and(|(s, t)| *s == seq && t == title);
        if unchanged && self.held_idle == 0 {
            return;
        }
        self.seen = Some((seq, title.to_string()));
        let screen = screen();
        let input = Input {
            screen: &screen,
            osc_title: title,
            osc_progress: "",
        };
        if let Some(reading) = read(agent, &input) {
            self.apply(reading);
        }
    }

    fn apply(&mut self, reading: Reading) {
        if reading.skip {
            return;
        }
        // Nothing on screen says the agent is busy: either a plain idle, or a
        // screen the manifest cannot place at all. Codex's idle prompt is the
        // second kind, since its manifest has no idle rule, so treating
        // "unknown" as "unchanged" kept a Codex that had finished booting on
        // working forever.
        let quiet = match reading.state {
            ScreenState::Idle => !reading.visible_idle,
            ScreenState::Unknown => true,
            ScreenState::Working | ScreenState::Blocked => false,
        };
        if self.status == Some(AgentStatus::Working) && quiet && self.held_idle < IDLE_CONFIRMATIONS
        {
            self.held_idle += 1;
            return;
        }
        self.held_idle = 0;
        // A rule that says "unknown" (a model picker, a transcript viewer) is
        // no evidence of anything, so it leaves the call to the hooks.
        self.matched = reading.matched && reading.state != ScreenState::Unknown;
        let finished = match self.status {
            Some(AgentStatus::Working | AgentStatus::Waiting | AgentStatus::Done) => {
                Some(AgentStatus::Done)
            }
            _ => Some(AgentStatus::Idle),
        };
        self.status = match reading.state {
            ScreenState::Working => Some(AgentStatus::Working),
            ScreenState::Blocked => Some(AgentStatus::Waiting),
            ScreenState::Idle => finished,
            // The busy states it could have left end the turn. A calm one it
            // cannot confirm stays as it was.
            ScreenState::Unknown => match self.status {
                Some(AgentStatus::Working | AgentStatus::Waiting) => finished,
                other => other,
            },
        };
    }

    /// The screen's status and whether a rule backed the last reading. `None`
    /// until the screen has said anything, and for agents with no manifest.
    pub(crate) fn status(&self) -> Option<(AgentStatus, bool)> {
        self.status.map(|s| (s, self.matched))
    }
}

/// The live bottom of the pane as text, in herdr's detection shape: one line per
/// row, blank cells as spaces, trailing blank rows dropped. On the primary screen
/// the window ends at the lower of the cursor and the last row with text, so a
/// short UI at the top of a tall pane is still in view. Scrolling the viewport
/// back does not move it.
pub(crate) fn detection_text<T: EventListener>(term: &Term<T>) -> String {
    use alacritty_terminal::grid::Dimensions as _;
    use alacritty_terminal::index::{Column, Line};
    use alacritty_terminal::term::TermMode;
    use alacritty_terminal::term::cell::Flags;

    let grid = term.grid();
    let rows = grid.screen_lines() as i32;
    let cols = grid.columns();
    if rows == 0 || cols == 0 {
        return String::new();
    }
    let row_text = |line: i32| -> String {
        let row = &grid[Line(line)];
        let mut text = String::with_capacity(cols);
        for col in 0..cols {
            let cell = &row[Column(col)];
            if cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
                continue;
            }
            text.push(cell.c);
            if let Some(zerowidth) = cell.zerowidth() {
                text.extend(zerowidth);
            }
        }
        text
    };
    let viewport: Vec<String> = (0..rows).map(row_text).collect();
    let end = match term.mode().contains(TermMode::ALT_SCREEN) {
        true => rows - 1,
        false => {
            let cursor = grid.cursor.point.line.0;
            viewport
                .iter()
                .rposition(|row| !row.trim().is_empty())
                .map_or(rows - 1, |last| (last as i32).max(cursor))
        }
    };
    let start = (end + 1 - rows).max(-(grid.history_size() as i32));
    let mut lines: Vec<String> = (start..=end)
        .map(|line| match line {
            0.. => viewport[line as usize].clone(),
            _ => row_text(line),
        })
        .collect();
    while lines.last().is_some_and(|row| row.trim().is_empty()) {
        lines.pop();
    }
    match lines.is_empty() {
        true => String::new(),
        false => lines.join("\n") + "\n",
    }
}

#[derive(Deserialize)]
struct Manifest {
    #[serde(default)]
    rules: Vec<Rule>,
}

#[derive(Deserialize)]
struct Rule {
    state: Option<ScreenState>,
    #[serde(default)]
    priority: i32,
    #[serde(default = "whole_recent")]
    region: String,
    #[serde(default)]
    visible_idle: bool,
    #[serde(default)]
    skip_state_update: bool,
    #[serde(flatten)]
    gate: Gate,
}

fn whole_recent() -> String {
    "whole_recent".to_string()
}

#[derive(Deserialize, Default)]
struct Gate {
    #[serde(default)]
    all: Vec<Gate>,
    #[serde(default)]
    any: Vec<Gate>,
    #[serde(default, rename = "not")]
    not_gate: Vec<Gate>,
    #[serde(default)]
    contains: Vec<String>,
    #[serde(default)]
    regex: Vec<String>,
    #[serde(default)]
    line_regex: Vec<String>,
}

struct CompiledManifest {
    rules: Vec<CompiledRule>,
}

struct CompiledRule {
    state: ScreenState,
    priority: i32,
    region: String,
    visible_idle: bool,
    skip_state_update: bool,
    gate: CompiledGate,
}

struct CompiledGate {
    all: Vec<CompiledGate>,
    any: Vec<CompiledGate>,
    not_gate: Vec<CompiledGate>,
    /// Already lowercased: `contains` is case-insensitive.
    contains: Vec<String>,
    regex: Vec<Regex>,
    line_regex: Vec<Regex>,
}

impl CompiledGate {
    fn compile(gate: &Gate) -> Result<CompiledGate, regex::Error> {
        let gates = |list: &[Gate]| {
            list.iter()
                .map(CompiledGate::compile)
                .collect::<Result<Vec<_>, _>>()
        };
        let regexes = |list: &[String]| {
            list.iter()
                .map(|p| Regex::new(p))
                .collect::<Result<Vec<_>, _>>()
        };
        Ok(CompiledGate {
            all: gates(&gate.all)?,
            any: gates(&gate.any)?,
            not_gate: gates(&gate.not_gate)?,
            contains: gate.contains.iter().map(|c| c.to_lowercase()).collect(),
            regex: regexes(&gate.regex)?,
            line_regex: regexes(&gate.line_regex)?,
        })
    }

    fn matches(&self, text: &str) -> bool {
        self.matches_with(text, &text.to_lowercase())
    }

    fn matches_with(&self, text: &str, lower: &str) -> bool {
        self.contains
            .iter()
            .all(|needle| lower.contains(needle.as_str()))
            && self.regex.iter().all(|re| re.is_match(text))
            && self
                .line_regex
                .iter()
                .all(|re| text.lines().any(|line| re.is_match(line)))
            && self.all.iter().all(|g| g.matches_with(text, lower))
            && (self.any.is_empty() || self.any.iter().any(|g| g.matches_with(text, lower)))
            && !self.not_gate.iter().any(|g| g.matches_with(text, lower))
    }
}

fn compile(source: &str) -> Result<CompiledManifest, String> {
    let manifest: Manifest = toml::from_str(source).map_err(|e| e.to_string())?;
    let rules = manifest
        .rules
        .iter()
        .map(|rule| {
            Ok(CompiledRule {
                state: rule.state.unwrap_or(ScreenState::Unknown),
                priority: rule.priority,
                region: rule.region.trim().to_string(),
                visible_idle: rule.visible_idle,
                skip_state_update: rule.skip_state_update,
                gate: CompiledGate::compile(&rule.gate).map_err(|e| e.to_string())?,
            })
        })
        .collect::<Result<_, String>>()?;
    Ok(CompiledManifest { rules })
}

fn compiled(agent: CLIAgent) -> Option<&'static CompiledManifest> {
    static CACHE: [OnceLock<Option<CompiledManifest>>; CLIAgent::ALL.len()] =
        [const { OnceLock::new() }; CLIAgent::ALL.len()];
    let slot = CLIAgent::ALL.iter().position(|a| *a == agent)?;
    CACHE[slot]
        .get_or_init(|| {
            let source = manifest_source(agent)?;
            compile(source)
                .inspect_err(|e| log::warn!("{} screen manifest: {e}", agent.slug()))
                .ok()
        })
        .as_ref()
}

fn region<'a>(input: &Input<'a>, spec: &str) -> &'a str {
    let content = input.screen;
    match spec {
        "osc_title" => input.osc_title,
        "osc_progress" => input.osc_progress,
        "whole_recent" => content,
        "after_last_prompt_marker" => after_last_prompt_marker(content),
        "before_current_prompt_marker" => before_current_prompt_marker(content),
        "whole_recent_without_current_prompt_marker" => {
            match current_codex_prompt_index(&content.lines().collect::<Vec<_>>()) {
                Some(_) => "",
                None => content,
            }
        }
        "current_prompt_block_marker" => current_prompt_block_marker(content).unwrap_or(""),
        "after_current_prompt_block_marker" => {
            after_current_prompt_block_marker(content).unwrap_or("")
        }
        "prompt_box_body" => prompt_box_body(content).unwrap_or(""),
        "above_prompt_box" => above_prompt_box(content),
        "last_non_empty_above_prompt_box" => last_non_empty_line(above_prompt_box(content)),
        "after_last_horizontal_rule" => after_last_horizontal_rule(content),
        _ => {
            if let Some(n) = region_count(spec, "bottom_lines") {
                return bottom_lines(content, n);
            }
            if let Some(n) = region_count(spec, "bottom_non_empty_lines") {
                return bottom_non_empty_lines(content, n);
            }
            if let Some(n) = region_count(spec, "top_non_empty_lines") {
                return top_non_empty_lines(content, n);
            }
            ""
        }
    }
}

fn region_count(spec: &str, name: &str) -> Option<usize> {
    spec.strip_prefix(name)?
        .strip_prefix('(')?
        .strip_suffix(')')?
        .parse()
        .ok()
}

fn bottom_lines(content: &str, count: usize) -> &str {
    let lines: Vec<&str> = content.lines().collect();
    from_line(content, &lines, lines.len().saturating_sub(count))
}

fn bottom_non_empty_lines(content: &str, count: usize) -> &str {
    let lines: Vec<&str> = content.lines().collect();
    let start = lines
        .iter()
        .enumerate()
        .rev()
        .filter(|(_, line)| !line.trim().is_empty())
        .take(count)
        .last()
        .map(|(i, _)| i);
    match start {
        Some(start) => from_line(content, &lines, start),
        None => "",
    }
}

fn top_non_empty_lines(content: &str, count: usize) -> &str {
    let lines: Vec<&str> = content.lines().collect();
    let end = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .take(count)
        .last()
        .map(|(i, _)| i);
    match end {
        Some(end) => &content[..line_start(content, &lines, end + 1)],
        None => "",
    }
}

fn after_last_prompt_marker(content: &str) -> &str {
    let lines: Vec<&str> = content.lines().collect();
    match lines.iter().rposition(|line| codex_prompt_line(line)) {
        Some(i) => from_line(content, &lines, i + 1),
        None => content,
    }
}

fn before_current_prompt_marker(content: &str) -> &str {
    let lines: Vec<&str> = content.lines().collect();
    match current_codex_prompt_index(&lines) {
        Some(i) => &content[..line_start(content, &lines, i)],
        None => content,
    }
}

fn current_prompt_block_marker(content: &str) -> Option<&str> {
    let lines: Vec<&str> = content.lines().collect();
    let prompt = current_codex_prompt_index(&lines)?;
    lines[..prompt]
        .iter()
        .rev()
        .find(|line| codex_block_marker_line(line))
        .copied()
}

fn after_current_prompt_block_marker(content: &str) -> Option<&str> {
    let lines: Vec<&str> = content.lines().collect();
    let prompt = current_codex_prompt_index(&lines)?;
    let block = lines[..prompt]
        .iter()
        .rposition(|line| codex_block_marker_line(line))?;
    Some(from_line(content, &lines, block))
}

fn current_codex_prompt_index(lines: &[&str]) -> Option<usize> {
    let prompt = lines.iter().rposition(|line| codex_prompt_line(line))?;
    match lines[prompt + 1..]
        .iter()
        .any(|line| codex_block_marker_line(line))
    {
        true => None,
        false => Some(prompt),
    }
}

fn codex_prompt_line(line: &str) -> bool {
    line == "›" || line.starts_with("› ")
}

fn codex_block_marker_line(line: &str) -> bool {
    line.starts_with(['•', '■', '✗', '✓'])
}

fn prompt_box_body(content: &str) -> Option<&str> {
    let lines: Vec<&str> = content.lines().collect();
    let top = prompt_box_top_border(&lines)?;
    let start = line_start(content, &lines, top + 1);
    let end = lines[top + 1..]
        .iter()
        .position(|line| is_horizontal_rule(line))
        .map_or(lines.len(), |i| top + 1 + i);
    Some(&content[start..line_start(content, &lines, end).max(start)])
}

fn above_prompt_box(content: &str) -> &str {
    let lines: Vec<&str> = content.lines().collect();
    match prompt_box_top_border(&lines) {
        Some(top) => &content[..line_start(content, &lines, top)],
        None => content,
    }
}

fn after_last_horizontal_rule(content: &str) -> &str {
    let mut rule_end = 0;
    let mut offset = 0;
    for line in content.lines() {
        offset += line.len() + 1;
        if is_horizontal_rule(line) {
            rule_end = offset.min(content.len());
        }
    }
    &content[rule_end..]
}

fn last_non_empty_line(content: &str) -> &str {
    content
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("")
}

/// The upper of the last two horizontal rules: the top of the input box.
fn prompt_box_top_border(lines: &[&str]) -> Option<usize> {
    lines
        .iter()
        .enumerate()
        .rev()
        .filter(|(_, line)| is_horizontal_rule(line))
        .nth(1)
        .map(|(i, _)| i)
}

fn is_horizontal_rule(line: &str) -> bool {
    let trimmed = line.trim();
    let rule = trimmed.chars().take_while(|&c| c == '─').count();
    if rule == 0 {
        return false;
    }
    let rest = trimmed[rule * '─'.len_utf8()..].trim_start();
    rest.is_empty() || rule >= 3
}

fn from_line<'a>(content: &'a str, lines: &[&str], index: usize) -> &'a str {
    &content[line_start(content, lines, index)..]
}

/// Byte offset of line `index`, counting one byte per line break the way
/// herdr's snapshot writes them.
fn line_start(content: &str, lines: &[&str], index: usize) -> usize {
    lines[..index.min(lines.len())]
        .iter()
        .map(|line| line.len() + 1)
        .sum::<usize>()
        .min(content.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen(agent: CLIAgent, text: &str) -> Reading {
        read(
            agent,
            &Input {
                screen: text,
                osc_title: "",
                osc_progress: "",
            },
        )
        .expect("the agent has a manifest")
    }

    #[test]
    fn every_manifest_parses_and_compiles() {
        for agent in CLIAgent::ALL {
            if let Some(source) = manifest_source(agent) {
                let manifest =
                    compile(source).unwrap_or_else(|e| panic!("{}'s manifest: {e}", agent.slug()));
                assert!(!manifest.rules.is_empty(), "{} has no rules", agent.slug());
                for rule in &manifest.rules {
                    let known = matches!(
                        rule.region.as_str(),
                        "osc_title"
                            | "osc_progress"
                            | "whole_recent"
                            | "after_last_prompt_marker"
                            | "before_current_prompt_marker"
                            | "whole_recent_without_current_prompt_marker"
                            | "current_prompt_block_marker"
                            | "after_current_prompt_block_marker"
                            | "prompt_box_body"
                            | "above_prompt_box"
                            | "last_non_empty_above_prompt_box"
                            | "after_last_horizontal_rule"
                    ) || [
                        "bottom_lines",
                        "bottom_non_empty_lines",
                        "top_non_empty_lines",
                    ]
                    .iter()
                    .any(|name| region_count(&rule.region, name).is_some());
                    assert!(known, "{}: unknown region {}", agent.slug(), rule.region);
                }
            }
        }
    }

    #[test]
    fn antigravity_reads_its_permission_prompt_and_its_spinner() {
        let blocked = screen(
            CLIAgent::Antigravity,
            "  Requesting permission for: run_command\n  rm -rf build\n  Do you want to proceed?\n  1. Yes\n",
        );
        assert_eq!(blocked.state, ScreenState::Blocked);
        assert!(blocked.matched);

        let working = screen(
            CLIAgent::Antigravity,
            "> fix the test\n ⠋ Thinking (esc to cancel)\n",
        );
        assert_eq!(working.state, ScreenState::Working);

        let idle = screen(CLIAgent::Antigravity, "> \n  ? for shortcuts\n");
        assert_eq!(idle.state, ScreenState::Idle);
        assert!(!idle.matched, "an idle agy is the fallback, not a rule");
    }

    #[test]
    fn cursor_reads_its_stop_hint_and_its_approval_prompt() {
        let working = screen(
            CLIAgent::Cursor,
            "  Reading src/main.rs\n\n  ⬢ Generating\n  ctrl+c to stop\n",
        );
        assert_eq!(working.state, ScreenState::Working);

        let blocked = screen(
            CLIAgent::Cursor,
            "  Run this command?\n  $ cargo test\n  Waiting for approval\n  Run (once) (y)\n  Skip (esc or n)\n",
        );
        assert_eq!(blocked.state, ScreenState::Blocked);
    }

    #[test]
    fn codex_without_evidence_is_unknown_rather_than_idle() {
        let reading = screen(CLIAgent::Codex, "nothing recognisable\n");
        assert_eq!(reading.state, ScreenState::Unknown);
        assert!(!reading.matched);
    }

    #[test]
    fn claude_reads_its_title_spinner() {
        let reading = read(
            CLIAgent::Claude,
            &Input {
                screen: "",
                osc_title: "⠂ Fixing the build",
                osc_progress: "",
            },
        )
        .unwrap();
        assert_eq!(reading.state, ScreenState::Working);
        let reading = read(
            CLIAgent::Claude,
            &Input {
                screen: "",
                osc_title: "✳ Fixing the build",
                osc_progress: "",
            },
        )
        .unwrap();
        assert_eq!(reading.state, ScreenState::Idle);
        assert!(reading.visible_idle);
    }

    #[test]
    fn agents_herdr_has_no_manifest_for_read_nothing() {
        assert!(manifest_source(CLIAgent::Aider).is_none());
        let mut tracker = Tracker::default();
        tracker.observe(Some(CLIAgent::Aider), 1, "", || {
            panic!("no manifest, so nothing to render the screen for")
        });
        assert_eq!(tracker.status(), None);
    }

    fn reading(state: ScreenState, visible_idle: bool) -> Reading {
        Reading {
            state,
            matched: true,
            visible_idle,
            skip: false,
        }
    }

    #[test]
    fn idle_after_work_is_a_finished_turn() {
        let mut t = Tracker::default();
        t.apply(reading(ScreenState::Idle, false));
        assert_eq!(
            t.status(),
            Some((AgentStatus::Idle, true)),
            "nothing ran yet"
        );
        t.apply(reading(ScreenState::Working, false));
        t.apply(reading(ScreenState::Blocked, false));
        assert_eq!(t.status(), Some((AgentStatus::Waiting, true)));
        t.apply(reading(ScreenState::Idle, true));
        assert_eq!(t.status(), Some((AgentStatus::Done, true)));
        t.apply(reading(ScreenState::Idle, true));
        assert_eq!(
            t.status(),
            Some((AgentStatus::Done, true)),
            "done until the next turn"
        );
    }

    #[test]
    fn a_quiet_screen_mid_turn_must_repeat_before_it_ends_the_turn() {
        let mut t = Tracker::default();
        t.apply(reading(ScreenState::Working, false));
        for _ in 0..IDLE_CONFIRMATIONS {
            t.apply(reading(ScreenState::Idle, false));
            assert_eq!(t.status(), Some((AgentStatus::Working, true)));
        }
        t.apply(reading(ScreenState::Idle, false));
        assert_eq!(t.status(), Some((AgentStatus::Done, true)));

        let mut t = Tracker::default();
        t.apply(reading(ScreenState::Working, false));
        t.apply(reading(ScreenState::Idle, true));
        assert_eq!(
            t.status(),
            Some((AgentStatus::Done, true)),
            "an idle prompt on screen needs no second look"
        );
    }

    #[test]
    fn an_unchanged_screen_is_not_read_twice() {
        let mut t = Tracker::default();
        let reads = std::cell::Cell::new(0);
        let screen = || {
            reads.set(reads.get() + 1);
            "> \n".to_string()
        };
        t.observe(Some(CLIAgent::Antigravity), 7, "agy", screen);
        t.observe(Some(CLIAgent::Antigravity), 7, "agy", screen);
        assert_eq!(reads.get(), 1);
        t.observe(Some(CLIAgent::Antigravity), 8, "agy", screen);
        assert_eq!(reads.get(), 2);
    }

    fn session(status: AgentStatus, rich: bool) -> AgentSessionState {
        AgentSessionState {
            status,
            rich,
            ..AgentSessionState::default()
        }
    }

    #[test]
    fn screen_evidence_outranks_hooks_that_can_miss_a_prompt() {
        let hook = session(AgentStatus::Working, true);
        assert_eq!(
            merge(
                CLIAgent::Codex,
                Some(&hook),
                Some((AgentStatus::Waiting, true))
            ),
            Some(AgentStatus::Waiting),
            "codex's hooks never report a permission prompt"
        );
        assert_eq!(
            merge(
                CLIAgent::Claude,
                Some(&hook),
                Some((AgentStatus::Idle, false))
            ),
            Some(AgentStatus::Working),
            "a fallback idle is no reason to overrule a reporting hook"
        );
        assert_eq!(
            merge(CLIAgent::Kimi, Some(&hook), Some((AgentStatus::Done, true))),
            Some(AgentStatus::Working),
            "kimi's hooks cover the whole turn, as herdr trusts them to"
        );
        let done = session(AgentStatus::Done, true);
        assert_eq!(
            merge(
                CLIAgent::Claude,
                Some(&done),
                Some((AgentStatus::Idle, true))
            ),
            Some(AgentStatus::Done),
            "a turn that ended between two looks at the screen still ended"
        );
    }

    #[test]
    fn codex_back_at_its_prompt_is_no_longer_working() {
        // Codex boots behind a spinner in its title, then sits at a prompt that
        // its manifest has no rule for — herdr's reading there is "unknown".
        let fallback = Reading {
            state: ScreenState::Unknown,
            matched: false,
            visible_idle: false,
            skip: false,
        };
        let mut t = Tracker::default();
        t.apply(reading(ScreenState::Working, false));
        for _ in 0..=IDLE_CONFIRMATIONS {
            t.apply(fallback);
        }
        assert_ne!(
            t.status().map(|(s, _)| s),
            Some(AgentStatus::Working),
            "the spinner is gone, so the turn is over"
        );
        assert_ne!(
            merge(CLIAgent::Codex, None, t.status()),
            Some(AgentStatus::Working),
            "and with no hook reporting yet, nothing else can keep it working"
        );
        let hook = session(AgentStatus::Idle, true);
        assert_eq!(
            merge(CLIAgent::Codex, Some(&hook), t.status()),
            Some(AgentStatus::Idle),
            "a hook that has reported gets the call"
        );
    }

    #[test]
    fn a_screen_the_manifest_calls_unknown_defers_to_the_hooks() {
        // Claude's model picker is a rule whose state is "unknown".
        let mut t = Tracker::default();
        t.apply(reading(ScreenState::Idle, true));
        t.apply(reading(ScreenState::Unknown, false));
        assert_eq!(t.status(), Some((AgentStatus::Idle, false)));
        let hook = session(AgentStatus::Waiting, true);
        assert_eq!(
            merge(CLIAgent::Claude, Some(&hook), t.status()),
            Some(AgentStatus::Waiting)
        );
    }

    #[test]
    fn with_no_hooks_the_screen_is_all_there_is() {
        assert_eq!(
            merge(
                CLIAgent::Antigravity,
                None,
                Some((AgentStatus::Idle, false))
            ),
            Some(AgentStatus::Idle)
        );
        assert_eq!(
            merge(CLIAgent::Antigravity, None, None),
            None,
            "not read yet"
        );
        let notified = session(AgentStatus::Waiting, false);
        assert_eq!(
            merge(CLIAgent::Aider, Some(&notified), None),
            Some(AgentStatus::Waiting),
            "a bare notification still counts where there is no screen reading"
        );
    }

    #[test]
    fn regions_follow_herdr() {
        let text = "a\n\nb\nc\n\n";
        assert_eq!(bottom_non_empty_lines(text, 2), "b\nc\n\n");
        assert_eq!(bottom_lines(text, 2), "c\n\n");
        assert_eq!(top_non_empty_lines(text, 2), "a\n\nb\n");
        let boxed = "out\n────────\n> type here\n────────\nhint\n";
        assert_eq!(prompt_box_body(boxed), Some("> type here\n"));
        assert_eq!(above_prompt_box(boxed), "out\n");
        assert_eq!(after_last_horizontal_rule(boxed), "hint\n");
        let codex = "• Ran tests\n› \n";
        assert_eq!(current_prompt_block_marker(codex), Some("• Ran tests"));
        assert_eq!(before_current_prompt_marker(codex), "• Ran tests\n");
    }

    fn term_with(rows: usize, bytes: &[u8]) -> Term<alacritty_terminal::event::VoidListener> {
        use alacritty_terminal::term::Config;
        use alacritty_terminal::vte::ansi::Processor;
        struct Size(usize);
        impl alacritty_terminal::grid::Dimensions for Size {
            fn total_lines(&self) -> usize {
                self.0
            }
            fn screen_lines(&self) -> usize {
                self.0
            }
            fn columns(&self) -> usize {
                20
            }
        }
        let mut term = Term::new(
            Config::default(),
            &Size(rows),
            alacritty_terminal::event::VoidListener,
        );
        let mut parser: Processor = Processor::new();
        parser.advance(&mut term, bytes);
        term
    }

    #[test]
    fn detection_text_ends_at_the_live_bottom() {
        let term = term_with(4, b"one\r\ntwo\r\n");
        assert_eq!(
            detection_text(&term),
            format!("{:20}\n{:20}\n", "one", "two"),
            "rows keep their width, and the empty rows under the cursor go"
        );

        let term = term_with(3, b"1\r\n2\r\n3\r\n4\r\n5");
        let text = detection_text(&term);
        assert!(text.contains('5') && text.contains('3'));
        assert!(!text.contains('2'), "only a screenful: {text:?}");
    }

    #[test]
    fn detection_text_skips_the_spacer_after_a_wide_glyph() {
        let term = term_with(2, "界x".as_bytes());
        assert!(detection_text(&term).starts_with("界x"));
    }
}

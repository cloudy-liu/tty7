//! The mark an agent avatar wears in its corner.
//!
//! Herdr's five states and its "distinct symbols" style, drawn the way herdr
//! draws them: blocked `×`, working `◐`, done `✓`, idle `○`, unknown `·`. The
//! old badge was a nine-pixel dot that told working from done by hue alone and
//! drew nothing at all for idle, so an idle agent and one whose status tty7
//! could not read looked the same. Every state is a different shape here as
//! well as a different colour.

use crate::core::cli_agent::AgentStatus;
use crate::ui::i18n::{L10nKey, t};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusIndicator {
    /// Stopped on a question or a permission prompt — the one worth acting on.
    Blocked,
    Working,
    /// A finished turn nobody has looked at yet.
    Done,
    Idle,
    /// An agent tty7 recognised but has no status for: no hooks installed and
    /// no screen manifest, or neither has reported yet.
    Unknown,
}

impl StatusIndicator {
    pub const ALL: [StatusIndicator; 5] = [
        StatusIndicator::Blocked,
        StatusIndicator::Working,
        StatusIndicator::Done,
        StatusIndicator::Idle,
        StatusIndicator::Unknown,
    ];

    /// What an agent pane shows, given its status (hooks and screen together,
    /// `None` when neither has said anything) and whether its last result is
    /// unread.
    ///
    /// Herdr keeps a finished turn on "done" only until you look at it, then
    /// calls the agent idle again — so a read `Done` is `Idle` here.
    pub fn of_agent(status: Option<AgentStatus>, unread: bool) -> StatusIndicator {
        match status {
            None => StatusIndicator::Unknown,
            Some(AgentStatus::Done) if !unread => StatusIndicator::Idle,
            Some(status) => StatusIndicator::of_status(status),
        }
    }

    /// The indicator for a bare status, where nothing says whether a finished
    /// turn has been read (the tray's agent list).
    pub fn of_status(status: AgentStatus) -> StatusIndicator {
        match status {
            AgentStatus::Waiting => StatusIndicator::Blocked,
            AgentStatus::Working => StatusIndicator::Working,
            AgentStatus::Done => StatusIndicator::Done,
            AgentStatus::Idle => StatusIndicator::Idle,
        }
    }

    pub fn icon_path(self) -> &'static str {
        match self {
            StatusIndicator::Blocked => "icons/status/blocked.svg",
            StatusIndicator::Working => "icons/status/working.svg",
            StatusIndicator::Done => "icons/status/done.svg",
            StatusIndicator::Idle => "icons/status/idle.svg",
            StatusIndicator::Unknown => "icons/status/unknown.svg",
        }
    }

    /// Herdr's colours: red, yellow, teal, green and overlay0 from Catppuccin
    /// Mocha on a dark surface and Catppuccin Latte on a light one — the pair
    /// herdr ships as its default dark and light themes.
    pub fn rgb(self, dark: bool) -> u32 {
        match (self, dark) {
            (StatusIndicator::Blocked, true) => 0xF38BA8,
            (StatusIndicator::Working, true) => 0xF9E2AF,
            (StatusIndicator::Done, true) => 0x94E2D5,
            (StatusIndicator::Idle, true) => 0xA6E3A1,
            (StatusIndicator::Unknown, true) => 0x6C7086,
            (StatusIndicator::Blocked, false) => 0xD20F39,
            (StatusIndicator::Working, false) => 0xDF8E1D,
            (StatusIndicator::Done, false) => 0x179299,
            (StatusIndicator::Idle, false) => 0x40A02B,
            (StatusIndicator::Unknown, false) => 0x9CA0B0,
        }
    }

    /// The word the sidebar writes ahead of a row's second line — herdr's own
    /// English words in every locale, the way herdr writes them.
    pub fn word(self) -> &'static str {
        match self {
            StatusIndicator::Blocked => "blocked",
            StatusIndicator::Working => "working",
            StatusIndicator::Done => "done",
            StatusIndicator::Idle => "idle",
            StatusIndicator::Unknown => "unknown",
        }
    }

    /// The words behind the symbol, for a tooltip.
    pub fn label(self) -> &'static str {
        t(match self {
            StatusIndicator::Blocked => L10nKey::AgentStatusWaiting,
            StatusIndicator::Working => L10nKey::AgentStatusWorking,
            StatusIndicator::Done => L10nKey::AgentStatusDone,
            StatusIndicator::Idle => L10nKey::AgentStatusIdle,
            StatusIndicator::Unknown => L10nKey::AgentStatusUnknown,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_finished_turn_reads_done_only_until_it_is_seen() {
        assert_eq!(
            StatusIndicator::of_agent(Some(AgentStatus::Done), true),
            StatusIndicator::Done
        );
        assert_eq!(
            StatusIndicator::of_agent(Some(AgentStatus::Done), false),
            StatusIndicator::Idle
        );
    }

    #[test]
    fn an_agent_with_no_reported_status_is_unknown_not_idle() {
        assert_eq!(
            StatusIndicator::of_agent(None, false),
            StatusIndicator::Unknown
        );
        assert_eq!(
            StatusIndicator::of_agent(Some(AgentStatus::Idle), false),
            StatusIndicator::Idle
        );
    }

    #[test]
    fn waiting_on_the_user_is_herdrs_blocked() {
        for unread in [false, true] {
            assert_eq!(
                StatusIndicator::of_agent(Some(AgentStatus::Waiting), unread),
                StatusIndicator::Blocked
            );
            assert_eq!(
                StatusIndicator::of_agent(Some(AgentStatus::Working), unread),
                StatusIndicator::Working
            );
        }
    }

    #[test]
    fn every_state_is_its_own_shape_colour_and_words() {
        crate::ui::i18n::set_locale("en");
        for dark in [true, false] {
            let colours: std::collections::HashSet<u32> =
                StatusIndicator::ALL.iter().map(|s| s.rgb(dark)).collect();
            assert_eq!(colours.len(), StatusIndicator::ALL.len());
        }
        let icons: std::collections::HashSet<&str> =
            StatusIndicator::ALL.iter().map(|s| s.icon_path()).collect();
        assert_eq!(icons.len(), StatusIndicator::ALL.len());
        let labels: std::collections::HashSet<&str> =
            StatusIndicator::ALL.iter().map(|s| s.label()).collect();
        assert_eq!(labels.len(), StatusIndicator::ALL.len());
        let words: std::collections::HashSet<&str> =
            StatusIndicator::ALL.iter().map(|s| s.word()).collect();
        assert_eq!(words.len(), StatusIndicator::ALL.len());
    }
}

//! Non-agent terminal applications with a recognizable tab icon.

use serde::{Deserialize, Serialize};

use crate::core::cli_agent::command_argv;
use crate::core::osc::percent_decode;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ForegroundApp {
    Herdr,
}

impl ForegroundApp {
    pub fn from_process_name(name: &str) -> Option<Self> {
        let name = name.rsplit(['/', '\\']).next().unwrap_or(name);
        (name.eq_ignore_ascii_case("herdr") || name.eq_ignore_ascii_case("herdr.exe"))
            .then_some(Self::Herdr)
    }

    /// The client keeps OSC 133;C as transmitted. PowerShell escapes the
    /// entire command line, including spaces, quotes and path separators.
    pub fn from_command_mark(raw: &str) -> Option<Self> {
        let decoded = percent_decode(raw.as_bytes());
        let command = String::from_utf8_lossy(&decoded);
        let argv = command_argv(&command);
        Self::from_process_name(argv.first()?)
    }
}

#[cfg(test)]
mod tests {
    use super::ForegroundApp;

    #[test]
    fn detects_herdr_from_literal_and_powershell_command_marks() {
        for command in [
            "herdr",
            "herdr --remote",
            "herdr%20--remote",
            r#"& "C:\tools\herdr.exe""#,
            "%26%20%22C%3A%5Ctools%5Cherdr.exe%22",
        ] {
            assert_eq!(
                ForegroundApp::from_command_mark(command),
                Some(ForegroundApp::Herdr)
            );
        }
        for command in ["", "codex", "cat herdr", "herdr-helper", "herdr.txt"] {
            assert_eq!(ForegroundApp::from_command_mark(command), None);
        }
    }

    #[test]
    fn process_names_require_the_herdr_executable() {
        for name in [
            "herdr",
            "HERDR.EXE",
            r"C:\tools\herdr.exe",
            "/usr/bin/herdr",
        ] {
            assert_eq!(
                ForegroundApp::from_process_name(name),
                Some(ForegroundApp::Herdr)
            );
        }
        for name in ["herdr-server", "herdr.txt", "other.exe", ""] {
            assert_eq!(ForegroundApp::from_process_name(name), None);
        }
    }
}

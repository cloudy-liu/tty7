//! ConPTY's win32-input-mode (`CSI ? 9001 h`).
//!
//! Windows ConPTY asks the host to encode keys as `KEY_EVENT_RECORD`s
//! (`CSI Vk;Sc;Uc;Kd;Cs;Rc _`) so Shift+Enter is distinguishable from Enter.
//! WezTerm honours that request; tty7 used to ignore it, so Codex on Windows
//! saw Shift+Enter as a plain CR. Kitty remains the higher-priority encoding
//! when an application has negotiated it (Claude Code, Cursor, Agy).

/// Streaming scanner for DEC private mode 9001 (`CSI ? 9001 h/l`).
///
/// ConPTY emits the request in the first handshake, sometimes packed with
/// other modes (`CSI ? 1004;9001 h`), and a snapshot replay can split the
/// sequence across chunks. The scanner is the latch the key encoder reads.
#[derive(Default)]
pub(crate) struct Win32InputModeScanner {
    state: State,
    /// Bytes after CSI `[`, not including `[`.
    csi: Vec<u8>,
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum State {
    #[default]
    Text,
    Esc,
    Csi,
    Osc,
    OscEsc,
}

const MAX_CSI: usize = 64;

impl Win32InputModeScanner {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }

    pub(crate) fn feed(&mut self, bytes: &[u8], mut on_mode: impl FnMut(bool)) {
        for &b in bytes {
            match self.state {
                State::Text => {
                    if b == 0x1b {
                        self.state = State::Esc;
                    }
                }
                State::Esc => {
                    self.state = match b {
                        b'[' => {
                            self.csi.clear();
                            State::Csi
                        }
                        b']' => State::Osc,
                        _ => State::Text,
                    };
                }
                State::Csi => {
                    if (0x40..=0x7e).contains(&b) {
                        if matches!(b, b'h' | b'l') && csi_decset_contains_9001(&self.csi) {
                            on_mode(b == b'h');
                        }
                        self.state = State::Text;
                    } else if self.csi.len() < MAX_CSI {
                        self.csi.push(b);
                    } else {
                        self.state = State::Text;
                    }
                }
                State::Osc => {
                    if b == 0x07 {
                        self.state = State::Text;
                    } else if b == 0x1b {
                        self.state = State::OscEsc;
                    }
                }
                State::OscEsc => {
                    self.state = if b == b'\\' { State::Text } else { State::Osc };
                }
            }
        }
    }
}

fn csi_decset_contains_9001(params: &[u8]) -> bool {
    let Some(rest) = params.strip_prefix(b"?") else {
        return false;
    };
    rest.split(|&b| b == b';').any(|p| p == b"9001")
}

/// Win32 `KEY_EVENT_RECORD` encoding for a chord that legacy VT would collapse.
///
/// Only modified Enter is encoded: that is the chord ConPTY otherwise reports
/// as a plain `VK_RETURN`. Unmodified keys stay on the legacy byte path.
/// Press and release are both emitted — WezTerm does the same, and crossterm
/// on Windows matches `insert_newline` on press while ignoring release.
pub(crate) fn encode_win32_enter(shift: bool, alt: bool, control: bool) -> Vec<u8> {
    const VK_RETURN: u32 = 13;
    const SCAN_RETURN: u32 = 28;
    const UNICODE_CR: u32 = 13;
    const SHIFT_PRESSED: u32 = 0x10;
    const LEFT_ALT_PRESSED: u32 = 0x02;
    const LEFT_CTRL_PRESSED: u32 = 0x08;
    let cs = u32::from(shift) * SHIFT_PRESSED
        + u32::from(alt) * LEFT_ALT_PRESSED
        + u32::from(control) * LEFT_CTRL_PRESSED;
    let mut out = Vec::new();
    for down in [1u32, 0] {
        out.extend_from_slice(
            format!("\x1b[{VK_RETURN};{SCAN_RETURN};{UNICODE_CR};{down};{cs};1_").as_bytes(),
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{Win32InputModeScanner, csi_decset_contains_9001, encode_win32_enter};

    fn modes(bytes: &[u8]) -> Vec<bool> {
        let mut got = Vec::new();
        Win32InputModeScanner::new().feed(bytes, |on| got.push(on));
        got
    }

    #[test]
    fn latches_on_the_conpty_handshake() {
        assert_eq!(modes(b"\x1b[?9001h"), [true]);
        assert_eq!(modes(b"\x1b[?9001l"), [false]);
        assert_eq!(modes(b"\x1b[?1004h\x1b[?9001h"), [true]);
        assert_eq!(modes(b"\x1b[?1004;9001h"), [true]);
        assert_eq!(modes(b"\x1b[?9001;1004h"), [true]);
        assert_eq!(modes(b"\x1b[?1004l"), Vec::<bool>::new());
        assert_eq!(modes(b"\x1b[9001h"), Vec::<bool>::new(), "SM is not DECSET");
    }

    #[test]
    fn survives_a_split_sequence() {
        let mut s = Win32InputModeScanner::new();
        let mut got = Vec::new();
        s.feed(b"\x1b[?90", |on| got.push(on));
        s.feed(b"01h", |on| got.push(on));
        assert_eq!(got, [true]);
    }

    #[test]
    fn ignores_osc_payloads_that_happen_to_contain_the_digits() {
        assert!(modes(b"\x1b]0;9001h\x07").is_empty());
        assert_eq!(modes(b"\x1b]0;x\x1b\\\x1b[?9001h"), [true]);
    }

    #[test]
    fn decset_param_split_requires_the_question_mark() {
        assert!(csi_decset_contains_9001(b"?9001"));
        assert!(csi_decset_contains_9001(b"?1004;9001"));
        assert!(!csi_decset_contains_9001(b"9001"));
        assert!(!csi_decset_contains_9001(b"?1004"));
    }

    #[test]
    fn shift_enter_is_vk_return_with_shift_pressed() {
        assert_eq!(
            encode_win32_enter(true, false, false),
            b"\x1b[13;28;13;1;16;1_\x1b[13;28;13;0;16;1_".to_vec()
        );
    }

    #[test]
    fn ctrl_enter_uses_left_ctrl() {
        assert_eq!(
            encode_win32_enter(false, false, true),
            b"\x1b[13;28;13;1;8;1_\x1b[13;28;13;0;8;1_".to_vec()
        );
    }
}

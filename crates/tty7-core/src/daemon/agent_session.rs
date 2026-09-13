//! Identify the conversation a live agent has open, without choosing a recent
//! file from the workspace. Hooks and explicit resume arguments cover the
//! general case; Codex also exposes its session through open rollout files.

use std::path::{Path, PathBuf};

use crate::core::cli_agent::CLIAgent;

pub(super) fn probe(agent: CLIAgent, shell_pid: u32, foreground: Option<i32>) -> Option<String> {
    if agent != CLIAgent::Codex {
        return None;
    }
    let mut processes = super::procinfo::processes(shell_pid, foreground)
        .into_iter()
        .filter(|process| {
            CLIAgent::detect_from_argv(std::slice::from_ref(&process.name)) == Some(agent)
        });
    let process = processes.next()?;
    // Nested Codex processes belong to different conversations.
    if processes.next().is_some() {
        return None;
    }
    codex_session(&super::procinfo::open_files(process.pid))
}

fn uuid(value: &str) -> Option<&str> {
    (value.len() == 36 && uuid::Uuid::try_parse(value).is_ok()).then_some(value)
}

fn rollout_id(path: &Path) -> Option<&str> {
    let name = path
        .file_name()?
        .to_str()?
        .strip_prefix("rollout-")?
        .strip_suffix(".jsonl")?;
    let id = name.get(name.len().checked_sub(36)?..)?;
    (path.ancestors().nth(4)?.file_name()? == "sessions").then_some(())?;
    uuid(id)
}

fn codex_session(paths: &[PathBuf]) -> Option<String> {
    let mut rollouts = std::collections::HashSet::new();
    let mut locks = std::collections::HashSet::new();
    for path in paths {
        if let Some(id) = rollout_id(path) {
            rollouts.insert(id);
        }
        if path
            .parent()
            .and_then(Path::file_name)
            .is_some_and(|name| name == "thread-writer-locks")
            && let Some(id) = path
                .file_stem()
                .and_then(|name| name.to_str())
                .and_then(uuid)
        {
            locks.insert(id);
        }
    }
    // More than one writer can mean Codex subagents sharing the process.
    // A lock without a rollout can be a new, empty thread, not resumable history.
    if rollouts.len() != 1 || locks.len() > 1 {
        return None;
    }
    let id = *rollouts.iter().next()?;
    (locks.is_empty() || locks.contains(id)).then(|| id.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(windows)]
    fn windows_reads_files_held_by_the_requested_process() {
        let dir = tempfile::tempdir().unwrap();
        let id = "0199c3f2-1b0e-7c3a-9f21-6d4b8e2a5c17";
        let directory = dir.path().join("sessions/2026/09/13");
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join(format!("rollout-2026-09-13T15-00-00-{id}.jsonl"));
        let _open = std::fs::File::create(&path).unwrap();
        let path = std::fs::canonicalize(path).unwrap();
        let files: Vec<_> = super::super::procinfo::open_files(std::process::id())
            .into_iter()
            .filter(|open| open == &path)
            .collect();
        assert_eq!(codex_session(&files).as_deref(), Some(id));
    }

    #[test]
    fn only_the_processes_open_conversation_is_a_resume_target() {
        let id = "0199c3f2-1b0e-7c3a-9f21-6d4b8e2a5c17";
        let other = "0199c3f2-1b0e-7c3a-9f21-6d4b8e2a5c18";
        let rollout = |id| {
            PathBuf::from(format!(
                "profile/sessions/2026/09/13/rollout-2026-09-13T15-00-00-{id}.jsonl"
            ))
        };
        let lock = |id| PathBuf::from(format!("profile/thread-writer-locks/{id}.lock"));
        assert_eq!(codex_session(&[rollout(id), lock(id)]).as_deref(), Some(id));
        assert_eq!(codex_session(&[rollout(id)]).as_deref(), Some(id));
        assert_eq!(
            codex_session(&[lock(id)]),
            None,
            "an empty launch has no history"
        );
        assert_eq!(
            codex_session(&[rollout(id), rollout(other), lock(id)]),
            None
        );
        assert_eq!(codex_session(&[rollout(id), lock(other)]), None);
        assert_eq!(codex_session(&[rollout(id), lock(id), lock(other)]), None);
        assert_eq!(
            codex_session(&[PathBuf::from(format!("notes/rollout-{id}.jsonl"))]),
            None
        );
    }
}

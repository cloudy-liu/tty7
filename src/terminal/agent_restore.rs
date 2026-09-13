use gpui::{Context, Window};
use tty7_core::core::machine::AgentFacts;

use crate::core::cli_agent::{CLIAgent, command_argv};
use crate::ui::host_ops::HostOps;

use super::view::{AgentSessionChanged, TerminalView};

/// A known Aider history file being checked on its host, before launching.
pub(super) struct PendingAgentRestore {
    session_id: String,
    launch_argv: Vec<String>,
    prompt_cycle: u64,
    identity: std::rc::Rc<()>,
}

impl TerminalView {
    pub(crate) fn pending_agent_restore(&self) -> Option<AgentFacts> {
        self.agent_restore.as_ref().map(|pending| AgentFacts {
            agent: CLIAgent::Aider,
            session_id: Some(pending.session_id.clone()),
            launch_argv: Some(pending.launch_argv.clone()),
            restore_pending: false,
            unstarted: false,
            status: None,
        })
    }

    pub(crate) fn restore_agent(
        &mut self,
        agent: CLIAgent,
        session_id: Option<&str>,
        launch_argv: Option<&[String]>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.clear_pending_agent_restore(cx);
        let id = session_id.or_else(|| launch_argv.and_then(|argv| agent.resumed_session_id(argv)));
        let Some((id, command)) = id.and_then(|id| {
            agent
                .resume_command(id, launch_argv)
                .map(|command| (id, command))
        }) else {
            log::info!(
                "{} restore has no exact target; leaving a fresh shell",
                agent.display_name()
            );
            return;
        };
        if agent != CLIAgent::Aider {
            self.run_resumed_agent(agent, id, &command, cx);
            return;
        }

        // Aider creates a missing history file. A failed check leaves the
        // shell usable instead of silently creating a new conversation.
        let Some(host) = self
            .remote_context()
            .is_none()
            .then(|| self.host(cx))
            .flatten()
        else {
            log::warn!("Aider history cannot be checked on its host; leaving a fresh shell");
            return;
        };
        let cwd = self.spawnable_cwd();
        let id = id.to_string();
        let identity = std::rc::Rc::new(());
        self.agent_restore = Some(PendingAgentRestore {
            session_id: id.clone(),
            launch_argv: command_argv(&command),
            prompt_cycle: self.terminal.prompt_cycle().max(1),
            identity: identity.clone(),
        });
        HostOps::run_in(
            host,
            window,
            cx,
            move |host| {
                let path = std::path::PathBuf::from(&id);
                let path = if host.is_absolute(&path) {
                    path
                } else {
                    let cwd =
                        cwd.ok_or_else(|| std::io::Error::other("pane has no working directory"))?;
                    host.join(&cwd, &id)
                };
                if host.stat(&path)?.is_dir {
                    return Err(std::io::Error::other("history target is a directory"));
                }
                Ok(path.to_string_lossy().into_owned())
            },
            move |this, result: std::io::Result<String>, _, cx| {
                this.sync_pending_agent_restore(cx);
                if !this
                    .agent_restore
                    .as_ref()
                    .is_some_and(|pending| std::rc::Rc::ptr_eq(&pending.identity, &identity))
                {
                    return;
                }
                let pending = this.agent_restore.take().unwrap();
                match result {
                    Ok(path) => {
                        if let Some(command) =
                            CLIAgent::Aider.resume_command(&path, Some(&pending.launch_argv))
                        {
                            this.run_resumed_agent(CLIAgent::Aider, &path, &command, cx);
                        }
                    }
                    Err(error) => {
                        log::warn!("Aider history restore failed ({error}); leaving a fresh shell")
                    }
                }
                cx.emit(AgentSessionChanged);
                cx.notify();
            },
        );
    }

    pub(super) fn sync_pending_agent_restore(&mut self, cx: &mut Context<Self>) {
        let Some(pending) = self.agent_restore.as_ref() else {
            return;
        };
        let cycle = self.terminal.prompt_cycle();
        if self.terminal.foreground_agent().is_some()
            || cycle > pending.prompt_cycle
            || (cycle == pending.prompt_cycle
                && self.terminal.shell_active()
                && !self.terminal.at_prompt())
        {
            self.clear_pending_agent_restore(cx);
        }
    }

    pub(crate) fn clear_pending_agent_restore(&mut self, cx: &mut Context<Self>) {
        if self.agent_restore.take().is_some() {
            cx.emit(AgentSessionChanged);
            cx.notify();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::protocol::ClientMsg;
    use crate::daemon::transport::Stream;
    use gpui::{Entity, TestAppContext, VisualTestContext};
    use std::time::Duration;

    fn harness(cx: &mut TestAppContext) -> (Entity<TerminalView>, VisualTestContext, Stream) {
        let (app, mut vcx) = crate::ui::app::test_window::harness(cx);
        let (view, daemon) = app.update_in(&mut vcx, |_, window, cx| {
            crate::terminal::view::quiet_test_pane(1, window, cx)
        });
        daemon
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        (view, vcx, daemon)
    }

    fn wait_for(
        view: &Entity<TerminalView>,
        cx: &mut VisualTestContext,
        ready: impl Fn(&TerminalView) -> bool,
    ) {
        for _ in 0..200 {
            cx.run_until_parked();
            if view.read_with(cx, |view, _| ready(view)) {
                return;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        panic!("restore state did not settle");
    }

    fn next_command(daemon: &mut Stream) -> String {
        loop {
            if let ClientMsg::Input(bytes) = ClientMsg::read(daemon).unwrap() {
                return String::from_utf8(bytes).unwrap();
            }
        }
    }

    #[gpui::test]
    fn exact_ids_are_saved_before_native_agents_receive_the_resume_command(
        cx: &mut TestAppContext,
    ) {
        let id = "0199c3f2-1b0e-7c3a-9f21-6d4b8e2a5c17";
        for agent in CLIAgent::ALL {
            if agent == CLIAgent::Aider {
                continue;
            }
            let (view, mut vcx, mut daemon) = harness(cx);
            view.update_in(&mut vcx, |view, window, cx| {
                view.restore_agent(agent, Some(id), None, window, cx);
                assert!(view.pending_agent_restore().is_none());
                assert_eq!(
                    view.agent_session().unwrap().session_id.as_deref(),
                    Some(id)
                );
            });
            let command = next_command(&mut daemon);
            assert!(command.ends_with('\r'));
            assert_eq!(
                agent.resumed_session_id(&command_argv(command.trim_end())),
                Some(id)
            );
        }
    }

    #[gpui::test]
    fn missing_aider_history_leaves_a_shell_and_valid_history_resumes(cx: &mut TestAppContext) {
        let dir = tempfile::tempdir().unwrap();
        let history = dir.path().join("original chat.md");
        let missing = dir.path().join("missing.md");
        std::fs::write(&history, "# Original conversation\n").unwrap();
        for invalid in [missing.as_path(), dir.path()] {
            let (view, mut vcx, mut daemon) = harness(cx);
            view.update_in(&mut vcx, |view, window, cx| {
                view.restore_agent(
                    CLIAgent::Aider,
                    Some(invalid.to_str().unwrap()),
                    None,
                    window,
                    cx,
                );
            });
            wait_for(&view, &mut vcx, |view| {
                view.pending_agent_restore().is_none()
            });
            view.update_in(&mut vcx, |view, window, cx| {
                assert!(view.agent().is_none());
                assert!(view.agent_session().is_none());
                view.restore_agent(
                    CLIAgent::Aider,
                    Some(history.to_str().unwrap()),
                    None,
                    window,
                    cx,
                );
            });
            wait_for(&view, &mut vcx, |view| view.agent_session().is_some());
            assert_eq!(
                view.read_with(&vcx, |view, _| view.agent_session().unwrap().session_id),
                Some(history.to_string_lossy().into_owned())
            );
            let command = next_command(&mut daemon);
            assert_eq!(
                command_argv(command.trim_end()),
                vec![
                    "aider",
                    "--chat-history-file",
                    history.to_str().unwrap(),
                    "--restore-chat-history"
                ]
            );
        }
        assert!(!missing.exists());
    }

    #[gpui::test]
    fn an_obsolete_history_check_cannot_start_an_agent(cx: &mut TestAppContext) {
        let dir = tempfile::tempdir().unwrap();
        let history = dir.path().join("original.md");
        std::fs::write(&history, "# Original conversation\n").unwrap();
        let (view, mut vcx, _daemon) = harness(cx);
        let previous = view.update_in(&mut vcx, |view, window, cx| {
            view.restore_agent(
                CLIAgent::Aider,
                Some(history.to_str().unwrap()),
                None,
                window,
                cx,
            );
            let previous = std::rc::Rc::downgrade(&view.agent_restore.as_ref().unwrap().identity);
            view.restore_agent(CLIAgent::Aider, None, None, window, cx);
            previous
        });
        wait_for(&view, &mut vcx, |_| previous.upgrade().is_none());
        view.read_with(&vcx, |view, _| {
            assert!(view.agent_session().is_none());
            assert!(view.pending_agent_restore().is_none());
        });
    }
}

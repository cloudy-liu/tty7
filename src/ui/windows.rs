use gpui::{
    AnyWindowHandle, App, AppContext as _, BorrowAppContext as _, Bounds, Global, Styled as _,
    TitlebarOptions, WeakEntity, Window, WindowBounds, WindowDecorations, WindowOptions, point, px,
    size,
};
use gpui_component::{Root, TitleBar};

use crate::core::config::{Config, StartupMode};
use crate::core::session::{WorkspaceId, WorkspaceStore};
use crate::core::window_state::{WindowGeometry as _, WindowState};
use crate::ui::app::Tty7App;
use crate::ui::i18n::{L10nKey, t, t_fmt, t_plural};

const CASCADE_STEP: f32 = 28.0;

const DEFAULT_SIZE: (f32, f32) = (1440.0, 900.0);

/// Yield to the platform between restored background windows so the foreground
/// can paint and receive input before a large recovery set is constructed.
const STARTUP_WINDOW_STAGGER: std::time::Duration = std::time::Duration::from_millis(1);

/// The smallest window tty7 still holds its shape in.
///
/// Below this the chrome stops being chrome: the sidebar is at its 180px floor
/// with barely forty columns beside it, and the overlays the app drops on top —
/// the palette at 560 wide, the switcher wider still — have nowhere to land.
/// Every one of those could be made to shrink further, but a terminal this
/// small is not a window anyone is working in, and a floor is the honest fix.
const MIN_SIZE: (f32, f32) = (720.0, 520.0);

struct WindowEntry {
    workspace: WorkspaceId,
    handle: AnyWindowHandle,
    app: WeakEntity<Tty7App>,
}

#[derive(Default)]
pub struct WindowRegistry {
    windows: Vec<WindowEntry>,
}

impl Global for WindowRegistry {}

impl WindowRegistry {
    pub fn init(cx: &mut App) {
        cx.set_global(Self::default());
    }

    pub fn count(cx: &mut App) -> usize {
        Self::sweep(cx);
        cx.global::<Self>().windows.len()
    }

    pub fn open_windows(cx: &mut App) -> Vec<(WorkspaceId, WeakEntity<Tty7App>)> {
        Self::sweep(cx);
        cx.global::<Self>()
            .windows
            .iter()
            .map(|w| (w.workspace, w.app.clone()))
            .collect()
    }

    pub fn window_for(cx: &mut App, workspace: WorkspaceId) -> Option<AnyWindowHandle> {
        Self::sweep(cx);
        cx.global::<Self>()
            .windows
            .iter()
            .find(|w| w.workspace == workspace)
            .map(|w| w.handle)
    }

    pub fn most_recent(cx: &mut App) -> Option<WorkspaceId> {
        Self::sweep(cx);
        let active = WorkspaceStore::all(cx).active;
        let registry = cx.global::<Self>();
        active
            .filter(|id| registry.windows.iter().any(|w| w.workspace == *id))
            .or_else(|| registry.windows.first().map(|w| w.workspace))
    }

    pub fn most_recent_local(cx: &mut App) -> Option<WorkspaceId> {
        Self::sweep(cx);
        let views = WorkspaceStore::all(cx);
        let registry = cx.global::<Self>();
        let is_open_local = |id: WorkspaceId| {
            registry.windows.iter().any(|window| window.workspace == id)
                && views.get(id).is_some_and(|view| !view.is_remote())
        };
        views.active.filter(|id| is_open_local(*id)).or_else(|| {
            registry
                .windows
                .iter()
                .filter(|window| is_open_local(window.workspace))
                .max_by_key(|window| {
                    views
                        .get(window.workspace)
                        .map(|view| view.last_active)
                        .unwrap_or_default()
                })
                .map(|window| window.workspace)
        })
    }

    pub fn app_in(cx: &mut App, window: &Window) -> Option<gpui::Entity<Tty7App>> {
        Self::sweep(cx);
        let handle = window.window_handle();
        cx.global::<Self>()
            .windows
            .iter()
            .find(|w| w.handle == handle)
            .and_then(|w| w.app.upgrade())
    }

    pub fn app_for(cx: &mut App, workspace: WorkspaceId) -> Option<WeakEntity<Tty7App>> {
        Self::sweep(cx);
        cx.global::<Self>()
            .windows
            .iter()
            .find(|w| w.workspace == workspace)
            .map(|w| w.app.clone())
    }

    pub fn refresh_locale(cx: &mut App, except: Option<WorkspaceId>) {
        Self::sweep(cx);
        let windows: Vec<_> = cx
            .global::<Self>()
            .windows
            .iter()
            .filter(|entry| Some(entry.workspace) != except)
            .map(|entry| (entry.handle, entry.app.clone()))
            .collect();
        for (handle, app) in windows {
            let _ = handle.update(cx, |_, window, cx| {
                let _ = app.update(cx, |app, cx| app.refresh_locale_state(window, cx));
                window.refresh();
            });
        }
    }

    /// Rebuilds every window's shell inventory from disk.
    ///
    /// `custom_shells` is read while that inventory is assembled, and it lives
    /// in `config.json` — which hot-reloads. The menu it feeds has to follow the
    /// file there, or the one surface the feature has appears not to work until
    /// the app is restarted, while every other key in the same save takes hold
    /// at once.
    pub fn refresh_shells(cx: &mut App) {
        Self::sweep(cx);
        let apps: Vec<_> = cx
            .global::<Self>()
            .windows
            .iter()
            .map(|entry| entry.app.clone())
            .collect();
        for app in apps {
            let _ = app.update(cx, |app, cx| app.refresh_shells(cx));
        }
    }

    pub(crate) fn register(
        cx: &mut App,
        workspace: WorkspaceId,
        handle: AnyWindowHandle,
        app: WeakEntity<Tty7App>,
    ) {
        cx.global_mut::<Self>().windows.push(WindowEntry {
            workspace,
            handle,
            app,
        });
    }

    pub fn unregister(cx: &mut App, workspace: WorkspaceId) {
        cx.global_mut::<Self>()
            .windows
            .retain(|w| w.workspace != workspace);
    }

    pub fn rebind(cx: &mut App, from: WorkspaceId, to: WorkspaceId) {
        if let Some(entry) = cx
            .global_mut::<Self>()
            .windows
            .iter_mut()
            .find(|w| w.workspace == from)
        {
            entry.workspace = to;
        }
    }

    fn sweep(cx: &mut App) {
        let dead: Vec<WorkspaceId> = cx
            .global::<Self>()
            .windows
            .iter()
            .filter(|w| w.app.upgrade().is_none())
            .map(|w| w.workspace)
            .collect();
        if dead.is_empty() {
            return;
        }
        cx.global_mut::<Self>()
            .windows
            .retain(|w| !dead.contains(&w.workspace));
    }
}

pub fn open(cx: &mut App, workspace: Option<WorkspaceId>) {
    open_at(cx, workspace, None);
}

/// Reveals `workspace` and activates one of its tabs. The window may already be
/// open, may belong to this process but be behind, or may not exist yet — the
/// caller does not care which. The tab is named by id rather than position
/// because the caller read it out of the machine tree, not out of that window.
pub fn open_at_tab(cx: &mut App, workspace: WorkspaceId, tab: tty7_core::core::machine::TabId) {
    open_at(cx, Some(workspace), None);
    let Some(handle) = WindowRegistry::window_for(cx, workspace) else {
        return;
    };
    let Some(app) = WindowRegistry::app_for(cx, workspace) else {
        return;
    };
    let _ = handle.update(cx, |_, window, cx| {
        window.activate_window();
        let _ = app.update(cx, |this, cx| {
            // A window opened just now may still be hydrating its tabs, so the
            // request parks until the tab arrives rather than probing once.
            this.activate_tree_tab(tab, window, cx);
        });
    });
}

pub fn open_at(
    cx: &mut App,
    workspace: Option<WorkspaceId>,
    initial_cwd: Option<std::path::PathBuf>,
) {
    let _ = open_at_with_disposition(cx, workspace, initial_cwd, OpenDisposition::Normal);
}

fn open_at_with_disposition(
    cx: &mut App,
    workspace: Option<WorkspaceId>,
    initial_cwd: Option<std::path::PathBuf>,
    disposition: OpenDisposition,
) -> bool {
    if let Some(id) = workspace
        && let Some(handle) = WindowRegistry::window_for(cx, id)
    {
        if disposition != OpenDisposition::RestoreBackground {
            let _ = handle.update(cx, |_, window, _| window.activate_window());
        }
        return true;
    }

    let options = window_options(cx, workspace, disposition);
    let mut created: Option<gpui::Entity<Tty7App>> = None;
    let opened = cx.open_window(options, |window, cx| {
        let app = cx.new(|cx| match (disposition, workspace, initial_cwd.clone()) {
            (OpenDisposition::RestoreBackground, Some(id), _) => {
                Tty7App::for_workspace_in_background(id, window, cx)
            }
            (_, _, Some(cwd)) => Tty7App::for_workspace_at(workspace, Some(cwd), window, cx),
            _ => Tty7App::for_workspace(workspace, window, cx),
        });
        created = Some(app.clone());
        cx.new(|cx| Root::new(app, window, cx).bg(gpui::transparent_black()))
    });

    let handle = match opened {
        Ok(handle) => handle,
        Err(e) => {
            log::error!("failed to open window: {e}");
            return false;
        }
    };
    let Some(app) = created else {
        log::error!("opened a window but its Tty7App was never built; not registering");
        return false;
    };

    let id = app.read(cx).workspace;
    WindowRegistry::register(cx, id, handle.into(), app.downgrade());
    refresh_menu(cx);
    true
}

/// A named workspace is the one that gets the window: the CLI made it, knows
/// its id, and no other window would do. Everything else is `tty7 [PATH]`,
/// where the CLI has no opinion and this process picks.
pub fn open_named_workspace_from_cli(cx: &mut App, workspace: WorkspaceId) {
    cx.activate(true);
    open(cx, Some(workspace));
    if let Some(handle) = WindowRegistry::window_for(cx, workspace) {
        let _ = handle.update(cx, |_, window, _| window.activate_window());
    }
}

pub fn open_from_cli(cx: &mut App, path: Option<std::path::PathBuf>) {
    // Only the GUI process knows which of its windows was focused most recently.
    // The daemon deliberately routes to a process, then leaves window selection
    // to this registry.
    let workspace = if path.is_some() {
        WindowRegistry::most_recent_local(cx)
    } else {
        WindowRegistry::most_recent(cx)
    };
    let Some(workspace) = workspace else {
        open_missing_cli_window_with(cx, path, open_at);
        return;
    };
    let Some(handle) = WindowRegistry::window_for(cx, workspace) else {
        return;
    };
    let Some(app) = WindowRegistry::app_for(cx, workspace).and_then(|app| app.upgrade()) else {
        return;
    };

    cx.activate(true);
    let _ = handle.update(cx, move |_, window, cx| {
        if let Some(path) = path {
            app.update(cx, |app, cx| app.new_tab_at(path, window, cx));
        }
        window.activate_window();
    });
}

/// What a launch reopens: the workspace, and how many other open windows the
/// restore left detached. Their panes are still running — the count exists so
/// the launch can say so instead of letting them be forgotten (#597).
pub type RestoreTarget = (WorkspaceId, usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OpenDisposition {
    Normal,
    RestoreForeground,
    RestoreBackground,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StartupWindow {
    pub workspace: Option<WorkspaceId>,
    pub initial_cwd: Option<std::path::PathBuf>,
    pub disposition: OpenDisposition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StartupPlan {
    pub windows: Vec<StartupWindow>,
    pub detached: Option<RestoreTarget>,
}

pub(crate) fn restore_at_launch_with(
    cx: &mut App,
    daemon: impl Into<Option<tty7_core::daemon::spawn::DaemonStartup>>,
    restore_session: bool,
    path: Option<std::path::PathBuf>,
    open: impl FnOnce(&mut App, StartupPlan),
) {
    use tty7_core::daemon::spawn::DaemonStartup;
    let daemon = daemon.into();

    let plan = if !restore_session {
        WorkspaceStore::close_all_for_launch(cx);
        StartupPlan {
            windows: vec![StartupWindow {
                workspace: None,
                initial_cwd: path,
                disposition: OpenDisposition::RestoreForeground,
            }],
            detached: None,
        }
    } else {
        match daemon {
            Some(DaemonStartup::Reused) => {
                let restored = restore_target(cx, path.as_deref());
                StartupPlan {
                    windows: vec![StartupWindow {
                        workspace: restored.map(|(id, _)| id),
                        initial_cwd: path,
                        disposition: OpenDisposition::RestoreForeground,
                    }],
                    detached: restored,
                }
            }
            Some(DaemonStartup::Spawned) => {
                let views = WorkspaceStore::all(cx);
                let foreground = startup_foreground(views, path.is_some());
                let mut windows = vec![StartupWindow {
                    workspace: foreground,
                    initial_cwd: path,
                    disposition: OpenDisposition::RestoreForeground,
                }];
                let mut background = views
                    .open_views()
                    .filter(|view| Some(view.id) != foreground)
                    .collect::<Vec<_>>();
                background.sort_by_key(|view| std::cmp::Reverse(view.last_active));
                windows.extend(background.into_iter().map(|view| StartupWindow {
                    workspace: Some(view.id),
                    initial_cwd: None,
                    disposition: OpenDisposition::RestoreBackground,
                }));
                StartupPlan {
                    windows,
                    detached: None,
                }
            }
            None => {
                // No answering daemon means there is no evidence that detached
                // panes survived and no successful spawn that could hydrate all
                // windows. Open one conservatively without mutating the saved
                // open set, so a later successful launch can still recover it.
                let workspace = startup_foreground(WorkspaceStore::all(cx), path.is_some());
                StartupPlan {
                    windows: vec![StartupWindow {
                        workspace,
                        initial_cwd: path,
                        disposition: OpenDisposition::RestoreForeground,
                    }],
                    detached: None,
                }
            }
        }
    };
    open(cx, plan);
}

pub fn restore_at_launch(
    cx: &mut App,
    daemon: Option<tty7_core::daemon::spawn::DaemonStartup>,
    restore_session: bool,
    path: Option<std::path::PathBuf>,
) {
    restore_at_launch_with(cx, daemon, restore_session, path, |cx, plan| {
        let detached = plan.detached;
        open_startup_plan_with(cx, plan, |cx, request| {
            open_startup_window_with(cx, request, |cx, request| {
                open_at_with_disposition(
                    cx,
                    request.workspace,
                    request.initial_cwd,
                    request.disposition,
                )
            })
        });
        announce_detached_at_launch(cx, detached);
    });
}

fn open_startup_plan_with<F>(cx: &mut App, plan: StartupPlan, open: F)
where
    F: FnMut(&mut App, StartupWindow) -> bool + 'static,
{
    let mut windows = std::collections::VecDeque::from(plan.windows);
    let Some(foreground) = windows.pop_front() else {
        return;
    };
    let mut open = open;
    let mut pending_initial_cwd = foreground.initial_cwd.clone();
    let mut needs_foreground = !open(cx, foreground);
    if !needs_foreground {
        pending_initial_cwd = None;
    }
    if windows.is_empty() && pending_initial_cwd.is_none() {
        return;
    }
    cx.spawn(async move |cx| {
        while let Some(mut next) = windows.pop_front() {
            cx.background_executor().timer(STARTUP_WINDOW_STAGGER).await;
            cx.update(|cx| {
                if !startup_window_is_still_requested(cx, &next) {
                    return;
                }
                let promote = needs_foreground
                    && (pending_initial_cwd.is_none() || startup_window_is_local(cx, &next));
                if promote {
                    next.disposition = OpenDisposition::RestoreForeground;
                    next.initial_cwd = pending_initial_cwd.take();
                }
                let retry_cwd = next.initial_cwd.clone();
                let opened = open(cx, next);
                if promote {
                    needs_foreground = !opened;
                    if !opened {
                        pending_initial_cwd = retry_cwd;
                    }
                }
            });
        }
        if needs_foreground && pending_initial_cwd.is_some() {
            cx.background_executor().timer(STARTUP_WINDOW_STAGGER).await;
            cx.update(|cx| {
                let _ = open(
                    cx,
                    StartupWindow {
                        workspace: None,
                        initial_cwd: pending_initial_cwd.take(),
                        disposition: OpenDisposition::RestoreForeground,
                    },
                );
            });
        }
    })
    .detach();
}

fn startup_window_is_still_requested(cx: &App, request: &StartupWindow) -> bool {
    match request.workspace {
        None => true,
        Some(workspace) => WorkspaceStore::all(cx)
            .get(workspace)
            .is_some_and(|view| view.open),
    }
}

fn startup_window_is_local(cx: &App, request: &StartupWindow) -> bool {
    match request.workspace {
        None => true,
        Some(workspace) => WorkspaceStore::all(cx)
            .get(workspace)
            .is_some_and(|view| !view.is_remote()),
    }
}

fn open_startup_window_with(
    cx: &mut App,
    request: StartupWindow,
    open: impl FnOnce(&mut App, StartupWindow) -> bool,
) -> bool {
    let workspace = request.workspace;
    let opened = open(cx, request);
    if !opened && let Some(workspace) = workspace {
        WorkspaceStore::mark_restore_open_failed(cx, workspace);
    }
    opened
}

fn startup_foreground(
    views: &crate::core::session::WindowViews,
    requires_local: bool,
) -> Option<WorkspaceId> {
    let eligible = |view: &crate::core::session::WindowView| {
        view.open && (!requires_local || !view.is_remote())
    };
    views
        .active
        .filter(|id| views.get(*id).is_some_and(&eligible))
        .or_else(|| {
            views
                .open_views()
                .filter(|view| eligible(view))
                .max_by_key(|view| view.last_active)
                .map(|view| view.id)
        })
}

/// The workspace a launch reopens, if any.
///
/// A launch that carries a directory restores the last layout exactly like a
/// pathless one does, and the directory becomes another tab in it. The two used
/// to disagree: "Open in tty7" from Explorer, or `tty7 <PATH>`, skipped the
/// restore outright whenever no window was already up, so a folder opened that
/// way came back as one blank terminal with the previous tabs nowhere — still
/// live on the machine, but reachable only through the switcher.
///
/// What a path does change is that it will not follow the layout onto a remote
/// machine. The requested directory is a path on this computer and a remote
/// workspace has no business spawning it, so that case starts a fresh local
/// workspace — which is what every path-carrying launch used to do.
pub fn restore_target(cx: &mut App, path: Option<&std::path::Path>) -> Option<RestoreTarget> {
    if path.is_some() {
        let views = WorkspaceStore::all(cx);
        let candidate = views.workspace_to_restore()?;
        if views.get(candidate).is_some_and(|view| view.is_remote()) {
            return None;
        }
    }
    WorkspaceStore::restore_one(cx)
}

/// Tell the user about the workspaces a launch restored away. A desktop toast
/// would work, but the window is right there — and the notification names the
/// place the workspaces can be got back from.
pub fn announce_detached_at_launch(cx: &mut App, restored: Option<RestoreTarget>) {
    let Some((workspace, detached)) = restored else {
        return;
    };
    if detached == 0 {
        return;
    }
    let Some(handle) = WindowRegistry::window_for(cx, workspace) else {
        return;
    };
    let _ = handle.update(cx, |_, window, cx| {
        gpui_component::WindowExt::push_notification(
            window,
            t_plural(L10nKey::LaunchWorkspacesLeftRunning, detached, &[]),
            cx,
        );
    });
}

/// Opens a window after CLI routing reaches a GUI process with no live windows.
///
/// Both shapes of request follow the same restoration policy as normal startup;
/// see [`restore_target`] for the one way a path narrows it.
fn open_missing_cli_window_with(
    cx: &mut App,
    path: Option<std::path::PathBuf>,
    open: impl FnOnce(&mut App, Option<WorkspaceId>, Option<std::path::PathBuf>),
) {
    let restore = restore_target(cx, path.as_deref());
    open(cx, restore.map(|(id, _)| id), path);
    announce_detached_at_launch(cx, restore);
}

pub fn refresh_menu(cx: &mut App) {
    crate::ui::theme::set_menus(cx);
}

pub const MENU_SLOTS: usize = 9;

pub fn menu_order(cx: &App) -> Vec<(WorkspaceId, bool)> {
    let all = WorkspaceStore::all(cx);
    let mut open: Vec<_> = all.views.iter().filter(|w| w.open).collect();
    let mut closed: Vec<_> = all.views.iter().filter(|w| !w.open).collect();
    open.sort_by(|a, b| b.last_active.cmp(&a.last_active));
    closed.sort_by(|a, b| b.last_active.cmp(&a.last_active));
    open.into_iter()
        .map(|w| (w.id, true))
        .chain(closed.into_iter().map(|w| (w.id, false)))
        .take(MENU_SLOTS)
        .collect()
}

pub struct PaneCountQuery {
    route: crate::terminal::PaneRoute,
    claimed: Vec<u64>,
}

pub fn pane_count_query(cx: &App, workspace: WorkspaceId) -> Option<PaneCountQuery> {
    let ws = WorkspaceStore::all(cx).get(workspace)?;
    Some(PaneCountQuery {
        route: crate::ui::remote_workspace::pane_route_for(cx, workspace),
        claimed: crate::ui::machine_mirror::pane_ids(cx, ws)?,
    })
}

pub fn live_pane_count(q: &PaneCountQuery) -> Option<usize> {
    let PaneCountQuery { route, claimed } = q;
    if claimed.is_empty() {
        return Some(0);
    }
    match crate::terminal::RemoteTerminal::try_list_panes_on(route) {
        Ok(panes) => {
            let alive: std::collections::HashSet<u64> = panes
                .into_iter()
                .filter(|p| p.alive)
                .map(|p| p.pane_id)
                .collect();
            Some(claimed.iter().filter(|id| alive.contains(id)).count())
        }
        Err(_) if matches!(route, crate::terminal::PaneRoute::Local) => Some(0),
        Err(_) => None,
    }
}

pub fn confirm_and_stop(cx: &mut App, window: &mut Window, workspace: WorkspaceId) {
    confirm_destructive(cx, window, workspace, "Stop", stop_workspace);
}

pub fn confirm_and_delete(cx: &mut App, window: &mut Window, workspace: WorkspaceId) {
    confirm_destructive(cx, window, workspace, "Delete", delete_workspace);
}

fn destructive_detail(live: Option<usize>, verb: &str) -> String {
    match (live, verb) {
        (None, "Delete") => t(L10nKey::WindowDeleteUnreachable).to_string(),
        (None, _) => t(L10nKey::WindowStopUnreachable).to_string(),
        (Some(0), _) => t_plural(L10nKey::WindowStopShells, 0, &[]),
        (Some(n), "Delete") => t_plural(L10nKey::WindowDeleteShells, n, &[]),
        (Some(n), _) => t_plural(L10nKey::WindowStopShells, n, &[]),
    }
}

fn confirm_destructive(
    cx: &mut App,
    window: &mut Window,
    workspace: WorkspaceId,
    verb: &'static str,
    act: fn(&mut App, WorkspaceId),
) {
    let name = crate::ui::machine_mirror::display_name_for(cx, workspace)
        .unwrap_or_else(|| t(L10nKey::WindowThisWorkspace).to_string());
    let query = pane_count_query(cx, workspace);
    let handle = window.window_handle();

    cx.spawn(async move |cx| {
        let live = match query {
            Some(q) => {
                cx.background_spawn(async move { live_pane_count(&q) })
                    .await
            }
            None => None,
        };

        if live == Some(0) && verb == "Stop" {
            let _ = cx.update(|cx| act(cx, workspace));
            return;
        }

        let detail = destructive_detail(live, verb);
        let verb_key = if verb == "Delete" {
            L10nKey::WindowDelete
        } else {
            L10nKey::WindowStop
        };
        let verb_label = t(verb_key);
        let title = t_fmt(
            L10nKey::WindowConfirmTitle,
            &[("verb", verb_label), ("name", &name)],
        );
        let Ok(answer) = handle.update(cx, |_, window, cx| {
            window.prompt(
                gpui::PromptLevel::Warning,
                &title,
                Some(&detail),
                &crate::ui::confirm_answers(verb_label, t(L10nKey::Cancel)),
                cx,
            )
        }) else {
            return;
        };

        if let Ok(0) = answer.await {
            let _ = cx.update(|cx| act(cx, workspace));
        }
    })
    .detach();
}

pub fn stop_workspace(cx: &mut App, workspace: WorkspaceId) {
    let doomed = doomed_pane_ids(cx, workspace);
    stop_workspace_keeping(cx, workspace, doomed);
}

fn doomed_pane_ids(cx: &App, workspace: WorkspaceId) -> Vec<u64> {
    WorkspaceStore::all(cx)
        .get(workspace)
        .and_then(|ws| crate::ui::machine_mirror::pane_ids(cx, ws))
        .unwrap_or_default()
}

fn stop_workspace_keeping(cx: &mut App, workspace: WorkspaceId, ids: Vec<u64>) {
    let route = crate::ui::remote_workspace::pane_route_for(cx, workspace);
    let host = WorkspaceStore::all(cx)
        .get(workspace)
        .map(|w| w.host_id())
        .unwrap_or(crate::ui::host_ops::HostId::LOCAL);
    if !ids.is_empty() {
        let route = route.clone();
        cx.background_executor()
            .spawn(async move {
                for pane_id in ids {
                    crate::terminal::RemoteTerminal::kill_pane_on(&route, pane_id);
                }
            })
            .detach();
    }
    if cx
        .try_global::<crate::terminal::pane_liveness::PaneLivenessCache>()
        .is_some()
    {
        cx.update_global::<crate::terminal::pane_liveness::PaneLivenessCache, _>(|cache, _| {
            cache.invalidate(host)
        });
    }
    if let Some(app) = WindowRegistry::app_for(cx, workspace)
        && let Some(app) = app.upgrade()
    {
        app.read(cx).teardown_workspace_forwards(cx);
    }
    close_window_for(cx, workspace);
    WorkspaceStore::close_window(cx, workspace);
    refresh_menu(cx);
}

pub fn delete_workspace(cx: &mut App, workspace: WorkspaceId) {
    let remote = WorkspaceStore::remote_ref(cx, workspace);
    let doomed = delete_from_tree(cx, workspace);
    stop_workspace_keeping(cx, workspace, doomed);
    WorkspaceStore::remove(cx, workspace);
    if let Some(remote) = remote {
        scrub_host_snapshots(cx, &remote);
    }
    release_unused_hosts(cx);
    refresh_menu(cx);
}

/// Drops a deleted workspace's row from every window's machine-listing
/// snapshot. The snapshot is captured at connect time and merged into the
/// switcher every frame, deduped against the store — so with the store entry
/// just removed, nothing holds that row back any more and the workspace the
/// user deleted pops straight back into the panel as an adoptable machine row
/// until the next reconnect replaces the snapshot. `forget_workspace` must
/// *not* do this: forgetting keeps the machine's session, and re-discovering
/// it from the listing is that flow's whole point (#485).
fn scrub_host_snapshots(cx: &mut App, remote: &crate::core::session::RemoteRef) {
    let host = remote.host_id();
    let machine_ws = remote.workspace;
    for (_, app) in WindowRegistry::open_windows(cx) {
        let Some(app) = app.upgrade() else {
            continue;
        };
        app.update(cx, |app, cx| {
            if let Some(snapshot) = app.host_snapshots.get_mut(&host) {
                snapshot.rows.retain(|row| row.id != machine_ws);
                cx.notify();
            }
        });
    }
}

/// Drop a workspace's local bookmark without telling its machine anything.
/// `delete_workspace` goes through `fire_workspace_op`, which sends
/// `WorkspaceRemove` — and with a live link that really kills the remote
/// panes. Forgetting only stops tracking the entry here; the remote daemon
/// keeps the session, and the switcher re-discovers it from the machine's
/// own workspace list if a route to it ever exists again (#485).
pub fn forget_workspace(cx: &mut App, workspace: WorkspaceId) {
    crate::ui::tree_sync::forget(cx, workspace);
    WorkspaceStore::remove(cx, workspace);
    release_unused_hosts(cx);
    refresh_menu(cx);
}

/// The local entries routing through a profile, minus the ones a profile
/// deletion must not touch (#485): an entry with a live or in-flight link
/// keeps its bookmark — the link holds an authenticated connection rather
/// than a profile reference, and forgetting the entry would release the
/// link under any window still attached to the machine. An entry whose
/// window is still on screen keeps it too: `forget_workspace` drops the
/// store entry without closing the window, and a window whose workspace
/// the store has forgotten reads as local — its next tab would open a
/// local shell on what the user still sees as a remote box. Whatever
/// survives the deletion falls into the parked state instead, which is
/// where the entry can be dismissed deliberately.
pub fn cascade_for_profile(cx: &mut App, profile: uuid::Uuid) -> Vec<WorkspaceId> {
    WorkspaceStore::all(cx)
        .workspaces_via_profile(profile)
        .into_iter()
        .filter(|ws| {
            let host = WorkspaceStore::host_of(cx, *ws);
            !crate::ui::remote_workspace::link_alive_or_connecting(cx, host)
                && WindowRegistry::app_for(cx, *ws).is_none()
        })
        .collect()
}

fn delete_from_tree(cx: &mut App, workspace: WorkspaceId) -> Vec<u64> {
    let doomed = doomed_pane_ids(cx, workspace);
    crate::ui::tree_sync::fire_workspace_op(cx, workspace, |ws| {
        tty7_core::daemon::control::ControlRequest::WorkspaceRemove { workspace: ws }
    });
    crate::ui::tree_sync::forget(cx, workspace);
    doomed
}

fn release_unused_hosts(cx: &mut App) {
    let live: Vec<_> = WorkspaceStore::all(cx)
        .views
        .iter()
        .filter(|w| w.is_remote())
        .map(|w| w.host_id())
        .collect();
    for id in crate::ui::host_registry::HostRegistry::ids(cx) {
        if !id.is_local() && !live.contains(&id) {
            crate::ui::remote_connect::HostLinks::remove(cx, id);
        }
    }
}

fn close_window_for(cx: &mut App, workspace: WorkspaceId) {
    let showing = WindowRegistry::app_for(cx, workspace);
    let Some(handle) = WindowRegistry::window_for(cx, workspace) else {
        return;
    };
    let Some(app) = showing.and_then(|weak| weak.upgrade()) else {
        return;
    };

    if WindowRegistry::count(cx) > 1 {
        WindowRegistry::unregister(cx, workspace);
        let _ = handle.update(cx, |_, window, _| window.remove_window());
        return;
    }

    let fresh = WorkspaceStore::claim(cx, None);
    WindowRegistry::rebind(cx, workspace, fresh);
    let _ = handle.update(cx, |_, window, cx| {
        app.update(cx, |app, cx| {
            app.adopt_workspace(fresh, crate::core::session::Session::default(), window, cx)
        });
    });
}

fn window_options(
    cx: &mut App,
    workspace: Option<WorkspaceId>,
    disposition: OpenDisposition,
) -> WindowOptions {
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    static APP_ICON: std::sync::LazyLock<Option<std::sync::Arc<image::RgbaImage>>> =
        std::sync::LazyLock::new(|| {
            image::load_from_memory(include_bytes!("../../assets/app-icon.png"))
                .ok()
                .map(|image| std::sync::Arc::new(image.thumbnail(256, 256).into_rgba8()))
        });

    let remember = cx.global::<Config>().remember_window_size;
    let remembered = remember
        .then(|| {
            workspace
                .and_then(|id| WorkspaceStore::all(cx).get(id).and_then(|w| w.window))
                .or_else(WindowState::load)
        })
        .flatten();
    let has_remembered_bounds = remembered.is_some();

    let existing = WindowRegistry::count(cx);
    let bounds = match remembered {
        Some(state) => {
            let bounds = at_least_min_size(state.bounds());
            if cx.displays().iter().any(|d| d.bounds().intersects(&bounds)) {
                bounds
            } else {
                Bounds::centered(None, bounds.size, cx)
            }
        }
        None => Bounds::centered(None, size(px(DEFAULT_SIZE.0), px(DEFAULT_SIZE.1)), cx),
    };
    let bounds = bounds_for_disposition(bounds, existing, disposition);

    let window_bounds = match cx.global::<Config>().startup_mode {
        _ if existing > 0 || (has_remembered_bounds && disposition != OpenDisposition::Normal) => {
            WindowBounds::Windowed(bounds)
        }
        StartupMode::Normal => WindowBounds::Windowed(bounds),
        StartupMode::Maximized => WindowBounds::Maximized(bounds),
        StartupMode::Fullscreen => WindowBounds::Fullscreen(bounds),
    };

    WindowOptions {
        window_bounds: Some(window_bounds),
        app_id: Some("tty7".to_owned()),
        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        icon: APP_ICON.as_ref().cloned(),
        titlebar: Some(TitlebarOptions {
            traffic_light_position: Some(crate::ui::theme::traffic_light_position()),
            ..TitleBar::title_bar_options()
        }),
        // The title bar above is tty7's own, so the compositor must not add a
        // second one: left unset, gpui asks Wayland for server-side
        // decorations (#679). Only the Linux backends read this — macOS and
        // Windows ignore the request — and X11 falls back to the window
        // manager's frame on its own when no compositor is running.
        window_decorations: Some(WindowDecorations::Client),
        window_background: crate::ui::theme::background_appearance(cx),
        window_min_size: Some(size(px(MIN_SIZE.0), px(MIN_SIZE.1))),
        focus: disposition != OpenDisposition::RestoreBackground,
        show: true,
        ..Default::default()
    }
}

fn bounds_for_disposition(
    bounds: Bounds<gpui::Pixels>,
    existing: usize,
    disposition: OpenDisposition,
) -> Bounds<gpui::Pixels> {
    match disposition {
        OpenDisposition::Normal => cascade(bounds, existing),
        OpenDisposition::RestoreForeground | OpenDisposition::RestoreBackground => bounds,
    }
}

/// Grow a remembered bound back up to `MIN_SIZE`.
///
/// `window_min_size` only governs what a *drag* may do to a window; the bounds
/// we open with are taken as given. A window that got under the minimum — an
/// older build that had no minimum, a hand-edited `views.json`, a display that
/// went away — was reopened at whatever it had been saved at, and the settings
/// page it opened onto had never been laid out for that width.
fn at_least_min_size(bounds: Bounds<gpui::Pixels>) -> Bounds<gpui::Pixels> {
    Bounds {
        origin: bounds.origin,
        size: size(
            bounds.size.width.max(px(MIN_SIZE.0)),
            bounds.size.height.max(px(MIN_SIZE.1)),
        ),
    }
}

fn cascade(bounds: Bounds<gpui::Pixels>, existing: usize) -> Bounds<gpui::Pixels> {
    if existing == 0 {
        return bounds;
    }
    let step = (existing % 5) as f32 * CASCADE_STEP;
    Bounds {
        origin: bounds.origin + point(px(step), px(step)),
        size: bounds.size,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::session::{RemoteRef, RemoteTarget, WindowView, WindowViews};
    use crate::ui::i18n::{L10nKey, set_locale, t_plural};

    fn bounds_at(x: f32, y: f32) -> Bounds<gpui::Pixels> {
        Bounds {
            origin: point(px(x), px(y)),
            size: size(px(800.), px(600.)),
        }
    }

    #[test]
    fn the_first_window_is_not_cascaded() {
        let b = bounds_at(100., 100.);
        assert_eq!(cascade(b, 0).origin, b.origin);
    }

    #[test]
    fn each_extra_window_steps_down_and_right() {
        let b = bounds_at(100., 100.);
        assert_eq!(
            cascade(b, 1).origin,
            point(px(100. + CASCADE_STEP), px(100. + CASCADE_STEP))
        );
        assert_eq!(
            cascade(b, 2).origin,
            point(px(100. + 2. * CASCADE_STEP), px(100. + 2. * CASCADE_STEP))
        );
        assert_eq!(cascade(b, 3).size, b.size);
    }

    #[test]
    fn a_remembered_bound_under_the_minimum_is_grown_back_to_it() {
        // The window in the report this was fixed for: 641x830, saved and
        // reopened at a width no settings page is laid out for.
        let undersized = Bounds {
            origin: point(px(700.), px(60.)),
            size: size(px(641.), px(830.)),
        };
        let grown = at_least_min_size(undersized);
        assert_eq!(grown.size.width, px(MIN_SIZE.0));
        assert_eq!(grown.size.height, px(830.), "a tall enough height is kept");
        assert_eq!(grown.origin, undersized.origin, "the corner does not move");

        let short = at_least_min_size(Bounds {
            origin: point(px(0.), px(0.)),
            size: size(px(1200.), px(300.)),
        });
        assert_eq!(short.size, size(px(1200.), px(MIN_SIZE.1)));
    }

    #[test]
    fn a_remembered_bound_at_or_over_the_minimum_is_left_alone() {
        let b = bounds_at(100., 100.);
        assert_eq!(at_least_min_size(b).size, b.size);
        let exact = Bounds {
            origin: point(px(0.), px(0.)),
            size: size(px(MIN_SIZE.0), px(MIN_SIZE.1)),
        };
        assert_eq!(at_least_min_size(exact).size, exact.size);
    }

    #[test]
    fn cascade_wraps_so_windows_never_march_off_screen() {
        let b = bounds_at(100., 100.);
        assert_eq!(cascade(b, 5).origin, b.origin);
        assert_eq!(cascade(b, 6).origin, cascade(b, 1).origin);
    }

    #[gpui::test]
    fn a_window_asks_the_compositor_for_client_side_decorations(cx: &mut gpui::TestAppContext) {
        // #679: tty7 draws its own title bar, so the request must say so —
        // gpui's default asks Wayland for a server-side frame on top of it.
        cx.update(|cx| {
            WindowRegistry::init(cx);
            let mut config = Config::default();
            config.remember_window_size = false;
            cx.set_global(config);

            let options = window_options(cx, None, OpenDisposition::Normal);
            assert_eq!(options.window_decorations, Some(WindowDecorations::Client));
            assert!(
                options.titlebar.is_some_and(|t| t.appears_transparent),
                "the in-app title bar is what the client-side request stands in for"
            );
        });
    }

    #[gpui::test]
    fn background_restore_is_visible_unfocused_and_does_not_cascade_saved_geometry(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(|cx| {
            WindowRegistry::init(cx);
            let mut config = Config::default();
            config.remember_window_size = true;
            config.startup_mode = StartupMode::Maximized;
            cx.set_global(config);

            let mut view = WindowView::default();
            let id = view.id;
            view.window = Some(WindowState {
                x: 100.0,
                y: 100.0,
                width: 800.0,
                height: 600.0,
            });
            WorkspaceStore::install_for_test(
                cx,
                WindowViews {
                    views: vec![view],
                    active: Some(id),
                },
            );

            let options = window_options(cx, Some(id), OpenDisposition::RestoreBackground);
            assert!(options.show, "restored workspaces are real visible windows");
            assert!(
                !options.focus,
                "background recovery must not steal typing focus"
            );
            assert!(
                matches!(options.window_bounds, Some(WindowBounds::Windowed(_))),
                "saved restoration geometry takes precedence over generic startup mode"
            );
            assert_eq!(
                options.window_bounds.map(|bounds| bounds.get_bounds()),
                Some(bounds_at(100.0, 100.0)),
                "saved placement is used exactly"
            );
            assert_eq!(
                bounds_for_disposition(
                    bounds_at(100.0, 100.0),
                    2,
                    OpenDisposition::RestoreBackground
                ),
                bounds_at(100.0, 100.0),
                "restoration never applies the ordinary new-window cascade"
            );
        });
    }

    #[gpui::test]
    fn a_pathless_cli_request_restores_a_workspace_when_no_window_is_open(
        cx: &mut gpui::TestAppContext,
    ) {
        let view = WindowView::default();
        let restored = view.id;
        let mut opened = None;

        cx.update(|cx| {
            WorkspaceStore::install_for_test(
                cx,
                WindowViews {
                    views: vec![view],
                    active: Some(restored),
                },
            );
            open_missing_cli_window_with(cx, None, |_, workspace, path| {
                opened = Some((workspace, path));
            });
        });

        assert_eq!(opened, Some((Some(restored), None)));
    }

    #[gpui::test]
    fn a_new_daemon_restores_every_window_with_the_active_one_first(cx: &mut gpui::TestAppContext) {
        let first = WindowView::default();
        let active = WindowView::default();
        let last = WindowView::default();
        let (first_id, active_id, last_id) = (first.id, active.id, last.id);
        let mut planned = None;

        cx.update(|cx| {
            WorkspaceStore::install_for_test(
                cx,
                WindowViews {
                    views: vec![first, active, last],
                    active: Some(active_id),
                },
            );
            restore_at_launch_with(
                cx,
                tty7_core::daemon::spawn::DaemonStartup::Spawned,
                true,
                None,
                |_, plan| planned = Some(plan),
            );
        });

        let plan = planned.expect("startup produces a window plan");
        assert_eq!(
            plan.windows,
            vec![
                StartupWindow {
                    workspace: Some(active_id),
                    initial_cwd: None,
                    disposition: OpenDisposition::RestoreForeground,
                },
                StartupWindow {
                    workspace: Some(first_id),
                    initial_cwd: None,
                    disposition: OpenDisposition::RestoreBackground,
                },
                StartupWindow {
                    workspace: Some(last_id),
                    initial_cwd: None,
                    disposition: OpenDisposition::RestoreBackground,
                },
            ]
        );
        assert_eq!(plan.detached, None);
        cx.update(|cx| {
            assert!(
                WorkspaceStore::all(cx).open_views().all(|view| view.open),
                "a new daemon needs every formerly open workspace to hydrate"
            );
        });
    }

    #[gpui::test]
    fn a_reused_daemon_keeps_the_single_window_policy(cx: &mut gpui::TestAppContext) {
        let first = WindowView::default();
        let active = WindowView::default();
        let last = WindowView::default();
        let (first_id, active_id, last_id) = (first.id, active.id, last.id);
        let mut planned = None;

        cx.update(|cx| {
            WorkspaceStore::install_for_test(
                cx,
                WindowViews {
                    views: vec![first, active, last],
                    active: Some(active_id),
                },
            );
            restore_at_launch_with(
                cx,
                tty7_core::daemon::spawn::DaemonStartup::Reused,
                true,
                None,
                |_, plan| planned = Some(plan),
            );
        });

        let plan = planned.expect("startup produces a window plan");
        assert_eq!(
            plan.windows,
            vec![StartupWindow {
                workspace: Some(active_id),
                initial_cwd: None,
                disposition: OpenDisposition::RestoreForeground,
            }]
        );
        assert_eq!(plan.detached, Some((active_id, 2)));
        cx.update(|cx| {
            let views = WorkspaceStore::all(cx);
            assert!(views.get(active_id).is_some_and(|view| view.open));
            assert!(views.get(first_id).is_some_and(|view| !view.open));
            assert!(views.get(last_id).is_some_and(|view| !view.open));
        });
    }

    #[gpui::test]
    fn a_daemon_startup_failure_neither_detaches_nor_claims_other_workspaces_are_served(
        cx: &mut gpui::TestAppContext,
    ) {
        let first = WindowView::default();
        let active = WindowView::default();
        let (first_id, active_id) = (first.id, active.id);
        let mut planned = None;

        cx.update(|cx| {
            WorkspaceStore::install_for_test(
                cx,
                WindowViews {
                    views: vec![first, active],
                    active: Some(active_id),
                },
            );
            restore_at_launch_with(
                cx,
                Option::<tty7_core::daemon::spawn::DaemonStartup>::None,
                true,
                None,
                |_, plan| planned = Some(plan),
            );
        });

        let plan = planned.expect("startup produces a conservative window plan");
        assert_eq!(plan.windows.len(), 1);
        assert_eq!(plan.windows[0].workspace, Some(active_id));
        assert_eq!(plan.detached, None, "continuity was not proven");
        cx.update(|cx| {
            let views = WorkspaceStore::all(cx);
            assert!(views.get(first_id).is_some_and(|view| view.open));
            assert!(views.get(active_id).is_some_and(|view| view.open));
        });
    }

    #[gpui::test]
    fn disabling_restore_opens_fresh_and_leaves_saved_workspaces_closed(
        cx: &mut gpui::TestAppContext,
    ) {
        let first = WindowView::default();
        let second = WindowView::default();
        let mut planned = None;

        cx.update(|cx| {
            WorkspaceStore::install_for_test(
                cx,
                WindowViews {
                    views: vec![first, second],
                    active: None,
                },
            );
            restore_at_launch_with(
                cx,
                tty7_core::daemon::spawn::DaemonStartup::Spawned,
                false,
                None,
                |_, plan| planned = Some(plan),
            );
        });

        let plan = planned.expect("startup produces a window plan");
        assert_eq!(
            plan.windows,
            vec![StartupWindow {
                workspace: None,
                initial_cwd: None,
                disposition: OpenDisposition::RestoreForeground,
            }]
        );
        assert_eq!(plan.detached, None);
        cx.update(|cx| {
            assert_eq!(WorkspaceStore::all(cx).open_views().count(), 0);
        });
    }

    #[gpui::test]
    fn a_directory_uses_a_local_foreground_and_still_restores_remote_windows(
        cx: &mut gpui::TestAppContext,
    ) {
        let mut local = WindowView::default();
        local.last_active = 10;
        let remote = WindowView::on_remote(RemoteRef::new(
            RemoteTarget::Wsl {
                distro: "Ubuntu".into(),
            },
            WorkspaceId::new(),
        ));
        let (local_id, remote_id) = (local.id, remote.id);
        let path = std::path::PathBuf::from("/tmp/project");
        let mut planned = None;

        cx.update(|cx| {
            WorkspaceStore::install_for_test(
                cx,
                WindowViews {
                    views: vec![local, remote],
                    active: Some(remote_id),
                },
            );
            restore_at_launch_with(
                cx,
                tty7_core::daemon::spawn::DaemonStartup::Spawned,
                true,
                Some(path.clone()),
                |_, plan| planned = Some(plan),
            );
        });

        assert_eq!(
            planned.expect("startup produces a window plan").windows,
            vec![
                StartupWindow {
                    workspace: Some(local_id),
                    initial_cwd: Some(path),
                    disposition: OpenDisposition::RestoreForeground,
                },
                StartupWindow {
                    workspace: Some(remote_id),
                    initial_cwd: None,
                    disposition: OpenDisposition::RestoreBackground,
                },
            ]
        );
    }

    #[gpui::test]
    fn a_new_daemon_does_not_reopen_workspaces_closed_before_the_crash(
        cx: &mut gpui::TestAppContext,
    ) {
        let mut closed = WindowView::default();
        closed.open = false;
        let open = WindowView::default();
        let open_id = open.id;
        let mut planned = None;

        cx.update(|cx| {
            WorkspaceStore::install_for_test(
                cx,
                WindowViews {
                    views: vec![closed, open],
                    active: None,
                },
            );
            restore_at_launch_with(
                cx,
                tty7_core::daemon::spawn::DaemonStartup::Spawned,
                true,
                None,
                |_, plan| planned = Some(plan),
            );
        });

        assert_eq!(
            planned.expect("startup produces a window plan").windows,
            vec![StartupWindow {
                workspace: Some(open_id),
                initial_cwd: None,
                disposition: OpenDisposition::RestoreForeground,
            }]
        );
    }

    #[gpui::test]
    fn a_new_daemon_falls_back_to_the_most_recent_open_workspace(cx: &mut gpui::TestAppContext) {
        let mut older = WindowView::default();
        older.last_active = 10;
        let mut newest = WindowView::default();
        newest.last_active = 20;
        let newest_id = newest.id;
        let closed_active = {
            let mut view = WindowView::default();
            view.open = false;
            view
        };
        let closed_id = closed_active.id;
        let mut planned = None;

        cx.update(|cx| {
            WorkspaceStore::install_for_test(
                cx,
                WindowViews {
                    views: vec![older, newest, closed_active],
                    active: Some(closed_id),
                },
            );
            restore_at_launch_with(
                cx,
                tty7_core::daemon::spawn::DaemonStartup::Spawned,
                true,
                None,
                |_, plan| planned = Some(plan),
            );
        });

        assert_eq!(
            planned.expect("startup produces a window plan").windows[0].workspace,
            Some(newest_id)
        );
    }

    #[gpui::test]
    fn no_saved_open_workspace_keeps_the_fresh_window_behavior(cx: &mut gpui::TestAppContext) {
        let mut closed = WindowView::default();
        closed.open = false;
        let mut planned = None;

        cx.update(|cx| {
            WorkspaceStore::install_for_test(
                cx,
                WindowViews {
                    views: vec![closed],
                    active: None,
                },
            );
            restore_at_launch_with(
                cx,
                tty7_core::daemon::spawn::DaemonStartup::Spawned,
                true,
                None,
                |_, plan| planned = Some(plan),
            );
        });

        assert_eq!(
            planned.expect("startup produces a window plan").windows,
            vec![StartupWindow {
                workspace: None,
                initial_cwd: None,
                disposition: OpenDisposition::RestoreForeground,
            }]
        );
    }

    #[gpui::test]
    fn a_directory_gets_a_fresh_local_foreground_when_only_remote_windows_were_open(
        cx: &mut gpui::TestAppContext,
    ) {
        let remote = WindowView::on_remote(RemoteRef::new(
            RemoteTarget::Wsl {
                distro: "Ubuntu".into(),
            },
            WorkspaceId::new(),
        ));
        let remote_id = remote.id;
        let path = std::path::PathBuf::from("/tmp/project");
        let mut planned = None;

        cx.update(|cx| {
            WorkspaceStore::install_for_test(
                cx,
                WindowViews {
                    views: vec![remote],
                    active: Some(remote_id),
                },
            );
            restore_at_launch_with(
                cx,
                tty7_core::daemon::spawn::DaemonStartup::Spawned,
                true,
                Some(path.clone()),
                |_, plan| planned = Some(plan),
            );
        });

        assert_eq!(
            planned.expect("startup produces a window plan").windows,
            vec![
                StartupWindow {
                    workspace: None,
                    initial_cwd: Some(path),
                    disposition: OpenDisposition::RestoreForeground,
                },
                StartupWindow {
                    workspace: Some(remote_id),
                    initial_cwd: None,
                    disposition: OpenDisposition::RestoreBackground,
                },
            ]
        );
    }

    #[gpui::test]
    fn background_restore_is_staged_and_continues_after_a_window_failure(
        cx: &mut gpui::TestAppContext,
    ) {
        use std::cell::RefCell;
        use std::rc::Rc;

        let first = WorkspaceId::new();
        let broken = WorkspaceId::new();
        let last = WorkspaceId::new();
        let opened = Rc::new(RefCell::new(Vec::new()));
        let opened_by_callback = opened.clone();

        cx.update(|cx| {
            WorkspaceStore::install_for_test(
                cx,
                WindowViews {
                    views: vec![
                        WindowView {
                            id: first,
                            ..WindowView::default()
                        },
                        WindowView {
                            id: broken,
                            ..WindowView::default()
                        },
                        WindowView {
                            id: last,
                            ..WindowView::default()
                        },
                    ],
                    active: Some(first),
                },
            );
            open_startup_plan_with(
                cx,
                StartupPlan {
                    windows: vec![
                        StartupWindow {
                            workspace: Some(first),
                            initial_cwd: None,
                            disposition: OpenDisposition::RestoreForeground,
                        },
                        StartupWindow {
                            workspace: Some(broken),
                            initial_cwd: None,
                            disposition: OpenDisposition::RestoreBackground,
                        },
                        StartupWindow {
                            workspace: Some(last),
                            initial_cwd: None,
                            disposition: OpenDisposition::RestoreBackground,
                        },
                    ],
                    detached: None,
                },
                move |cx, request| {
                    open_startup_window_with(cx, request, |_, request| {
                        opened_by_callback.borrow_mut().push(request.workspace);
                        request.workspace != Some(broken)
                    })
                },
            );
            assert_eq!(
                opened.borrow().as_slice(),
                &[Some(first)],
                "only the foreground opens in the initial effect cycle"
            );
        });

        assert_eq!(
            opened.borrow().as_slice(),
            &[Some(first)],
            "returning from the initial update is still before the first scheduler wake"
        );

        cx.executor().advance_clock(STARTUP_WINDOW_STAGGER);
        cx.run_until_parked();
        assert_eq!(
            opened.borrow().as_slice(),
            &[Some(first), Some(broken)],
            "one scheduler wake opens exactly one background window"
        );
        cx.update(|cx| {
            assert!(
                WorkspaceStore::all(cx)
                    .get(broken)
                    .is_some_and(|view| !view.open),
                "a failed restore is not left open without a registered window"
            );
        });

        cx.executor().advance_clock(STARTUP_WINDOW_STAGGER);
        cx.run_until_parked();
        assert_eq!(
            opened.borrow().as_slice(),
            &[Some(first), Some(broken), Some(last)],
            "a failed background open does not cancel later workspaces"
        );
    }

    #[gpui::test]
    fn staged_restore_skips_workspaces_closed_or_removed_before_their_wake(
        cx: &mut gpui::TestAppContext,
    ) {
        use std::cell::RefCell;
        use std::rc::Rc;

        let foreground = WorkspaceId::new();
        let closed = WorkspaceId::new();
        let removed = WorkspaceId::new();
        let remaining = WorkspaceId::new();
        let opened = Rc::new(RefCell::new(Vec::new()));
        let opened_by_callback = opened.clone();

        cx.update(|cx| {
            WorkspaceStore::install_for_test(
                cx,
                WindowViews {
                    views: [foreground, closed, removed, remaining]
                        .into_iter()
                        .map(|id| WindowView {
                            id,
                            ..WindowView::default()
                        })
                        .collect(),
                    active: Some(foreground),
                },
            );
            open_startup_plan_with(
                cx,
                StartupPlan {
                    windows: [
                        (foreground, OpenDisposition::RestoreForeground),
                        (closed, OpenDisposition::RestoreBackground),
                        (removed, OpenDisposition::RestoreBackground),
                        (remaining, OpenDisposition::RestoreBackground),
                    ]
                    .into_iter()
                    .map(|(workspace, disposition)| StartupWindow {
                        workspace: Some(workspace),
                        initial_cwd: None,
                        disposition,
                    })
                    .collect(),
                    detached: None,
                },
                move |_, request| {
                    opened_by_callback.borrow_mut().push(request.workspace);
                    true
                },
            );
            WorkspaceStore::close_window(cx, closed);
            WorkspaceStore::remove(cx, removed);
        });

        for _ in 0..2 {
            cx.executor().advance_clock(STARTUP_WINDOW_STAGGER);
            cx.run_until_parked();
            assert_eq!(
                opened.borrow().as_slice(),
                &[Some(foreground)],
                "a stale delayed request must not resurrect its workspace"
            );
        }
        cx.executor().advance_clock(STARTUP_WINDOW_STAGGER);
        cx.run_until_parked();
        assert_eq!(
            opened.borrow().as_slice(),
            &[Some(foreground), Some(remaining)],
            "valid later workspaces still restore after stale requests are skipped"
        );
    }

    #[gpui::test]
    fn a_failed_foreground_promotes_the_next_workspace(cx: &mut gpui::TestAppContext) {
        use std::cell::RefCell;
        use std::rc::Rc;

        let failed = WorkspaceId::new();
        let fallback = WorkspaceId::new();
        let opened = Rc::new(RefCell::new(Vec::new()));
        let opened_by_callback = opened.clone();

        cx.update(|cx| {
            WorkspaceStore::install_for_test(
                cx,
                WindowViews {
                    views: vec![
                        WindowView {
                            id: failed,
                            ..WindowView::default()
                        },
                        WindowView {
                            id: fallback,
                            ..WindowView::default()
                        },
                    ],
                    active: Some(failed),
                },
            );
            open_startup_plan_with(
                cx,
                StartupPlan {
                    windows: vec![
                        StartupWindow {
                            workspace: Some(failed),
                            initial_cwd: None,
                            disposition: OpenDisposition::RestoreForeground,
                        },
                        StartupWindow {
                            workspace: Some(fallback),
                            initial_cwd: None,
                            disposition: OpenDisposition::RestoreBackground,
                        },
                    ],
                    detached: None,
                },
                move |cx, request| {
                    open_startup_window_with(cx, request, |cx, request| {
                        opened_by_callback
                            .borrow_mut()
                            .push((request.workspace, request.disposition));
                        if request.workspace == Some(failed) {
                            false
                        } else {
                            WorkspaceStore::claim(cx, request.workspace);
                            true
                        }
                    })
                },
            );
        });

        assert_eq!(
            opened.borrow().as_slice(),
            &[(Some(failed), OpenDisposition::RestoreForeground)]
        );
        cx.update(|cx| {
            assert_eq!(WorkspaceStore::all(cx).active, None);
        });

        cx.executor().advance_clock(STARTUP_WINDOW_STAGGER);
        cx.run_until_parked();
        assert_eq!(
            opened.borrow().as_slice(),
            &[
                (Some(failed), OpenDisposition::RestoreForeground),
                (Some(fallback), OpenDisposition::RestoreForeground),
            ],
            "the next viable workspace becomes the focused window"
        );
        cx.update(|cx| {
            assert_eq!(WorkspaceStore::all(cx).active, Some(fallback));
        });
    }

    #[gpui::test]
    fn a_failed_active_workspace_promotes_the_most_recent_remaining_workspace(
        cx: &mut gpui::TestAppContext,
    ) {
        use std::cell::RefCell;
        use std::rc::Rc;

        let failed = WorkspaceId::new();
        let older = WorkspaceId::new();
        let newest = WorkspaceId::new();
        let opened = Rc::new(RefCell::new(Vec::new()));
        let opened_by_callback = opened.clone();

        cx.update(|cx| {
            WorkspaceStore::install_for_test(
                cx,
                WindowViews {
                    views: vec![
                        WindowView {
                            id: failed,
                            last_active: 30,
                            ..WindowView::default()
                        },
                        WindowView {
                            id: older,
                            last_active: 10,
                            ..WindowView::default()
                        },
                        WindowView {
                            id: newest,
                            last_active: 20,
                            ..WindowView::default()
                        },
                    ],
                    active: Some(failed),
                },
            );
            restore_at_launch_with(
                cx,
                tty7_core::daemon::spawn::DaemonStartup::Spawned,
                true,
                None,
                move |cx, plan| {
                    open_startup_plan_with(cx, plan, move |cx, request| {
                        open_startup_window_with(cx, request, |cx, request| {
                            opened_by_callback
                                .borrow_mut()
                                .push((request.workspace, request.disposition));
                            if request.workspace == Some(failed) {
                                false
                            } else {
                                WorkspaceStore::claim(cx, request.workspace);
                                true
                            }
                        })
                    });
                },
            );
        });

        cx.executor().advance_clock(STARTUP_WINDOW_STAGGER);
        cx.run_until_parked();
        assert_eq!(
            opened.borrow().as_slice(),
            &[
                (Some(failed), OpenDisposition::RestoreForeground),
                (Some(newest), OpenDisposition::RestoreForeground),
            ],
            "recency, not persistence order, chooses the fallback foreground"
        );
    }

    #[gpui::test]
    fn a_failed_path_foreground_keeps_remotes_background_and_carries_the_path_to_local(
        cx: &mut gpui::TestAppContext,
    ) {
        use std::cell::RefCell;
        use std::rc::Rc;

        let failed = WorkspaceId::new();
        let remote = WindowView::on_remote(RemoteRef::new(
            RemoteTarget::Wsl {
                distro: "Ubuntu".into(),
            },
            WorkspaceId::new(),
        ));
        let remote_id = remote.id;
        let local = WorkspaceId::new();
        let path = std::path::PathBuf::from("/tmp/project");
        let opened = Rc::new(RefCell::new(Vec::new()));
        let opened_by_callback = opened.clone();

        cx.update(|cx| {
            WorkspaceStore::install_for_test(
                cx,
                WindowViews {
                    views: vec![
                        WindowView {
                            id: failed,
                            ..WindowView::default()
                        },
                        remote,
                        WindowView {
                            id: local,
                            ..WindowView::default()
                        },
                    ],
                    active: Some(failed),
                },
            );
            open_startup_plan_with(
                cx,
                StartupPlan {
                    windows: vec![
                        StartupWindow {
                            workspace: Some(failed),
                            initial_cwd: Some(path.clone()),
                            disposition: OpenDisposition::RestoreForeground,
                        },
                        StartupWindow {
                            workspace: Some(remote_id),
                            initial_cwd: None,
                            disposition: OpenDisposition::RestoreBackground,
                        },
                        StartupWindow {
                            workspace: Some(local),
                            initial_cwd: None,
                            disposition: OpenDisposition::RestoreBackground,
                        },
                    ],
                    detached: None,
                },
                move |cx, request| {
                    open_startup_window_with(cx, request, |_, request| {
                        let failed_to_open = request.workspace == Some(failed);
                        opened_by_callback.borrow_mut().push(request);
                        !failed_to_open
                    })
                },
            );
        });

        cx.executor().advance_clock(STARTUP_WINDOW_STAGGER);
        cx.run_until_parked();
        cx.executor().advance_clock(STARTUP_WINDOW_STAGGER);
        cx.run_until_parked();
        assert_eq!(
            opened.borrow().as_slice(),
            &[
                StartupWindow {
                    workspace: Some(failed),
                    initial_cwd: Some(path.clone()),
                    disposition: OpenDisposition::RestoreForeground,
                },
                StartupWindow {
                    workspace: Some(remote_id),
                    initial_cwd: None,
                    disposition: OpenDisposition::RestoreBackground,
                },
                StartupWindow {
                    workspace: Some(local),
                    initial_cwd: Some(path),
                    disposition: OpenDisposition::RestoreForeground,
                },
            ],
            "the explicit local directory survives foreground failure"
        );
    }

    #[gpui::test]
    fn a_failed_path_foreground_creates_a_fresh_local_fallback_after_remote_restores(
        cx: &mut gpui::TestAppContext,
    ) {
        use std::cell::RefCell;
        use std::rc::Rc;

        let failed = WorkspaceId::new();
        let remote = WindowView::on_remote(RemoteRef::new(
            RemoteTarget::Wsl {
                distro: "Ubuntu".into(),
            },
            WorkspaceId::new(),
        ));
        let remote_id = remote.id;
        let path = std::path::PathBuf::from("/tmp/project");
        let opened = Rc::new(RefCell::new(Vec::new()));
        let opened_by_callback = opened.clone();

        cx.update(|cx| {
            WorkspaceStore::install_for_test(
                cx,
                WindowViews {
                    views: vec![
                        WindowView {
                            id: failed,
                            ..WindowView::default()
                        },
                        remote,
                    ],
                    active: Some(failed),
                },
            );
            open_startup_plan_with(
                cx,
                StartupPlan {
                    windows: vec![
                        StartupWindow {
                            workspace: Some(failed),
                            initial_cwd: Some(path.clone()),
                            disposition: OpenDisposition::RestoreForeground,
                        },
                        StartupWindow {
                            workspace: Some(remote_id),
                            initial_cwd: None,
                            disposition: OpenDisposition::RestoreBackground,
                        },
                    ],
                    detached: None,
                },
                move |cx, request| {
                    open_startup_window_with(cx, request, |_, request| {
                        let failed_to_open = request.workspace == Some(failed);
                        opened_by_callback.borrow_mut().push(request);
                        !failed_to_open
                    })
                },
            );
        });

        cx.executor().advance_clock(STARTUP_WINDOW_STAGGER);
        cx.run_until_parked();
        cx.executor().advance_clock(STARTUP_WINDOW_STAGGER);
        cx.run_until_parked();
        assert_eq!(
            opened.borrow().last(),
            Some(&StartupWindow {
                workspace: None,
                initial_cwd: Some(path),
                disposition: OpenDisposition::RestoreForeground,
            }),
            "without another saved local workspace, the path gets a fresh local window"
        );
    }

    #[gpui::test]
    fn a_pathless_cli_request_creates_a_default_window_without_history(
        cx: &mut gpui::TestAppContext,
    ) {
        let mut opened = None;

        cx.update(|cx| {
            WorkspaceStore::install_for_test(cx, WindowViews::default());
            open_missing_cli_window_with(cx, None, |_, workspace, path| {
                opened = Some((workspace, path));
            });
        });

        assert_eq!(opened, Some((None, None)));
    }

    #[gpui::test]
    fn a_request_carrying_a_path_restores_the_layout_and_brings_the_path_along(
        cx: &mut gpui::TestAppContext,
    ) {
        // The Explorer "Open in tty7" case with no window up: the folder is
        // wanted *and* so are the tabs that were there, which used to be
        // dropped on the floor by every launch that named a directory.
        let view = WindowView::default();
        let restored = view.id;
        let path = std::path::PathBuf::from("/tmp/somewhere");
        let mut opened = None;

        cx.update(|cx| {
            WorkspaceStore::install_for_test(
                cx,
                WindowViews {
                    views: vec![view],
                    active: Some(restored),
                },
            );
            open_missing_cli_window_with(cx, Some(path.clone()), |_, workspace, path| {
                opened = Some((workspace, path));
            });
        });

        assert_eq!(opened, Some((Some(restored), Some(path))));
    }

    #[gpui::test]
    fn a_path_will_not_follow_the_layout_onto_a_remote_machine(cx: &mut gpui::TestAppContext) {
        // The directory is a path on this computer, so a remote workspace is
        // the one restore candidate a path-carrying launch declines.
        let remote = WindowView::on_remote(RemoteRef::new(
            RemoteTarget::Wsl {
                distro: "Ubuntu".into(),
            },
            WorkspaceId::new(),
        ));
        let remote_id = remote.id;
        let path = std::path::PathBuf::from("/tmp/somewhere");
        let mut opened = None;

        cx.update(|cx| {
            WorkspaceStore::install_for_test(
                cx,
                WindowViews {
                    views: vec![remote],
                    active: Some(remote_id),
                },
            );
            open_missing_cli_window_with(cx, Some(path.clone()), |_, workspace, path| {
                opened = Some((workspace, path));
            });
        });

        assert_eq!(opened, Some((None, Some(path))));

        // Declining it is not the same as detaching it: the remote window is
        // left exactly as it was for the next pathless launch to restore.
        cx.update(|cx| {
            assert!(
                WorkspaceStore::all(cx)
                    .get(remote_id)
                    .is_some_and(|view| view.open),
                "a declined remote candidate must not be closed on its behalf"
            );
        });
    }

    #[test]
    fn the_confirmation_says_which_of_the_three_answers_it_got() {
        set_locale("en");
        assert_eq!(
            destructive_detail(Some(1), "Stop"),
            t_plural(L10nKey::WindowStopShells, 1, &[])
        );
        assert_eq!(
            destructive_detail(Some(3), "Stop"),
            t_plural(L10nKey::WindowStopShells, 3, &[])
        );
        assert_eq!(
            destructive_detail(Some(1), "Delete"),
            t_plural(L10nKey::WindowDeleteShells, 1, &[])
        );
        assert_eq!(
            destructive_detail(Some(3), "Delete"),
            t_plural(L10nKey::WindowDeleteShells, 3, &[])
        );
        assert_eq!(
            destructive_detail(Some(0), "Delete"),
            t_plural(L10nKey::WindowStopShells, 0, &[])
        );

        for verb in ["Stop", "Delete"] {
            let detail = destructive_detail(None, verb);
            assert!(
                detail.contains("could not be reached"),
                "{verb}: {detail:?} must say why there is no count"
            );
            assert!(
                !detail.contains("forgotten.") || verb == "Delete",
                "{verb}: {detail:?} promises a delete-only consequence"
            );
            assert!(
                !detail.chars().any(|c| c.is_ascii_digit()),
                "{verb}: {detail:?} states a count it does not have"
            );
        }
    }

    #[gpui::test]
    fn a_delete_reads_its_kill_list_before_the_removal_blanks_the_mirror(
        cx: &mut gpui::TestAppContext,
    ) {
        use tty7_core::core::machine::{Machine, PaneRecord, Tab, Workspace as TreeWorkspace};

        cx.update(|cx| {
            let view = WindowView::default();
            let id = view.id;
            WorkspaceStore::install_for_test(
                cx,
                WindowViews {
                    views: vec![view],
                    active: None,
                },
            );
            crate::ui::machine_mirror::MachineMirrors::install(
                cx,
                crate::ui::host_ops::HostId::LOCAL,
                Machine {
                    workspaces: vec![TreeWorkspace {
                        id,
                        tabs: vec![Tab::leaf(1), Tab::leaf(2), Tab::leaf(3)],
                        ..TreeWorkspace::default()
                    }],
                    panes: vec![PaneRecord::new(1), PaneRecord::new(2), PaneRecord::new(3)],
                },
            );

            let doomed = delete_from_tree(cx, id);
            assert_eq!(
                doomed,
                vec![1, 2, 3],
                "every shell the confirm prompt counted must be on the kill list"
            );
            assert!(
                doomed_pane_ids(cx, id).is_empty(),
                "the removal has been folded into the mirror — which is exactly why \
                 the list must be read first"
            );
        });
    }

    #[gpui::test]
    fn the_cascade_counts_only_the_deleted_profiles_unlinked_entries(
        cx: &mut gpui::TestAppContext,
    ) {
        // #485: deleting a profile forgets every entry routing through it —
        // its own entries, all of them, and nothing else's.
        cx.update(|cx| {
            WindowRegistry::init(cx);
            let doomed = uuid::Uuid::new_v4();
            let on_doomed_a = WindowView::on_remote(RemoteRef::new(
                RemoteTarget::Profile { id: doomed },
                WorkspaceId::new(),
            ));
            let on_doomed_b = WindowView::on_remote(RemoteRef::new(
                RemoteTarget::Profile { id: doomed },
                WorkspaceId::new(),
            ));
            let on_other = WindowView::on_remote(RemoteRef::new(
                RemoteTarget::Profile {
                    id: uuid::Uuid::new_v4(),
                },
                WorkspaceId::new(),
            ));
            let local = WindowView::default();
            let (a, b) = (on_doomed_a.id, on_doomed_b.id);
            WorkspaceStore::install_for_test(
                cx,
                WindowViews {
                    views: vec![on_doomed_a, on_doomed_b, on_other, local],
                    active: None,
                },
            );

            // No links exist in a test app, so nothing is excluded as
            // connected or connecting.
            let cascade = cascade_for_profile(cx, doomed);
            assert_eq!(cascade.len(), 2, "exactly the deleted profile's entries");
            assert!(cascade.contains(&a) && cascade.contains(&b));
            assert!(
                cascade_for_profile(cx, uuid::Uuid::new_v4()).is_empty(),
                "a profile with no entries cascades to nothing"
            );
        });
    }

    #[gpui::test]
    fn the_cascade_leaves_an_entry_whose_window_is_still_open(cx: &mut gpui::TestAppContext) {
        // #485: forgetting an entry does not close its window, and a window
        // the store has forgotten reads as local — so the one entry the user
        // is looking at must survive the cascade and park instead.
        use gpui::VisualContext as _;

        let (app, mut vcx) = crate::ui::app::test_window::harness(cx);
        let handle = vcx.window_handle();
        app.update_in(&mut vcx, |_, _, cx| {
            WindowRegistry::init(cx);
            let doomed = uuid::Uuid::new_v4();
            let onscreen = WindowView::on_remote(RemoteRef::new(
                RemoteTarget::Profile { id: doomed },
                WorkspaceId::new(),
            ));
            let offscreen = WindowView::on_remote(RemoteRef::new(
                RemoteTarget::Profile { id: doomed },
                WorkspaceId::new(),
            ));
            let (open, closed) = (onscreen.id, offscreen.id);
            WorkspaceStore::install_for_test(
                cx,
                WindowViews {
                    views: vec![onscreen, offscreen],
                    active: None,
                },
            );
            WindowRegistry::register(cx, open, handle, app.downgrade());

            let cascade = cascade_for_profile(cx, doomed);
            assert_eq!(
                cascade,
                vec![closed],
                "only the entry with no window on screen is forgotten"
            );
        });
    }

    #[gpui::test]
    fn forgetting_a_workspace_never_tells_its_machine(cx: &mut gpui::TestAppContext) {
        // #485: the profile-deletion cascade forgets rather than deletes —
        // `WorkspaceRemove` on a live link would kill the remote panes. The
        // machine mirror observes every op `fire_workspace_op` fires, so "the
        // mirror still lists the workspace" pins "no remove ever went out".
        use tty7_core::core::machine::{Machine, Workspace as TreeWorkspace};

        // `WorkspaceStore::remove` saves; a test has no business writing the
        // real views.
        let _ = tty7_core::core::config::set_config_dir(
            std::env::temp_dir().join(format!("tty7-windows-test-{}", std::process::id())),
        );
        cx.update(|cx| {
            let target = RemoteTarget::Profile {
                id: uuid::Uuid::new_v4(),
            };
            let machine_ws = WorkspaceId::new();
            let view = WindowView::on_remote(RemoteRef::new(target.clone(), machine_ws));
            let entry = view.id;
            let host = view.host_id();
            WorkspaceStore::install_for_test(
                cx,
                WindowViews {
                    views: vec![view],
                    active: None,
                },
            );
            crate::ui::machine_mirror::MachineMirrors::install(
                cx,
                host,
                Machine {
                    workspaces: vec![TreeWorkspace {
                        id: machine_ws,
                        ..TreeWorkspace::default()
                    }],
                    panes: vec![],
                },
            );

            forget_workspace(cx, entry);
            assert!(
                WorkspaceStore::all(cx).get(entry).is_none(),
                "the local bookmark is gone"
            );
            let machine = crate::ui::machine_mirror::MachineMirrors::machine(cx, host)
                .expect("the mirror itself stays");
            assert!(
                machine.workspaces.iter().any(|w| w.id == machine_ws),
                "forgetting fired no WorkspaceRemove — the machine keeps its session"
            );

            // Contrast: the delete path means it, and the mirror folds the
            // removal in even with the control link down.
            let view = WindowView::on_remote(RemoteRef::new(target, machine_ws));
            let entry = view.id;
            WorkspaceStore::install_for_test(
                cx,
                WindowViews {
                    views: vec![view],
                    active: None,
                },
            );
            delete_from_tree(cx, entry);
            let machine = crate::ui::machine_mirror::MachineMirrors::machine(cx, host).unwrap();
            assert!(
                !machine.workspaces.iter().any(|w| w.id == machine_ws),
                "delete_from_tree noted the remove it fired"
            );
        });
    }
}

use std::{collections::HashMap, path::PathBuf, sync::Arc};

use gpui::prelude::*;
use gpui::{
    App, BoxShadow, Context, Entity, FocusHandle, Focusable, FontWeight, Global, Hsla, Pixels,
    Render, ScrollHandle, SharedString, StyleRefinement, Subscription, Window, div, point, px,
    relative, rems,
};
use gpui_component::{
    ActiveTheme as _, ElementExt as _, IconName, Sizable as _, ThemeMode,
    button::{Button, ButtonVariants as _},
    highlighter::{HighlightTheme, HighlightThemeStyle, SyntaxColors, ThemeStyle},
    input::InputState,
    text::{TextView, TextViewState, TextViewStyle},
};

use crate::core::markdown_document::{self, Target};
use crate::core::{
    config::Config,
    markdown_theme::{self, Color, Entry, Registry, Snapshot, Theme},
};
use crate::ui::i18n::{L10nKey, t};
use crate::ui::{
    app::Tty7App,
    host_ops::{HostOps, SharedHost},
};
use gpui_component::{WindowExt as _, text::TextViewImageSource};

fn extensions() -> gpui_component::text::MarkdownExtensions {
    static EXTENSIONS: std::sync::OnceLock<gpui_component::text::MarkdownExtensions> =
        std::sync::OnceLock::new();
    EXTENSIONS
        .get_or_init(|| gpui_component::text::MarkdownExtensions::default().github_alerts())
        .clone()
}

impl Global for Registry {}

impl crate::ui::app::Tty7App {
    pub(crate) fn set_markdown_theme(&mut self, id: &str, cx: &mut Context<Self>) {
        if !cx
            .try_global::<Registry>()
            .is_some_and(|registry| registry.entries.contains_key(id))
        {
            return;
        }
        self.update_config(cx, |config| config.markdown_theme = id.to_owned());
        cx.refresh_windows();
    }
}

pub(crate) fn init(snapshot: Snapshot, cx: &mut App) {
    let mut registry = Registry::default();
    registry.replace(snapshot, &cx.global::<Config>().markdown_theme);
    cx.set_global(registry);
}

pub(crate) fn apply_snapshot(snapshot: Snapshot, cx: &mut App) {
    let selected = cx.global::<Config>().markdown_theme.clone();
    if cx.try_global::<Registry>().is_none() {
        cx.set_global(Registry::default());
    }
    cx.update_global::<Registry, _>(|registry, _| registry.replace(snapshot, &selected));
    cx.refresh_windows();
}

pub(crate) fn current(cx: &App) -> Entry {
    cx.try_global::<Registry>()
        .map(|registry| {
            registry
                .resolve(&cx.global::<Config>().markdown_theme)
                .clone()
        })
        .unwrap_or_else(|| Entry {
            theme: markdown_theme::builtin(),
            source: None,
            revision: 0,
        })
}

pub(crate) fn color(color: Color) -> Hsla {
    let [r, g, b, a] = color.0;
    gpui::Rgba { r, g, b, a }.into()
}

fn refinement(mut element: gpui::Div) -> StyleRefinement {
    element.style().clone()
}

fn highlight(theme: &Theme, dark: bool, revision: u64) -> Arc<HighlightTheme> {
    let palette = theme.palette(dark);
    let s = &palette.syntax;
    let token = |c| Some(ThemeStyle::from(color(c)));
    Arc::new(HighlightTheme {
        name: format!("{}:{}:{}", theme.id, dark, revision),
        appearance: if dark {
            ThemeMode::Dark
        } else {
            ThemeMode::Light
        },
        style: HighlightThemeStyle {
            editor_background: Some(color(palette.code_background)),
            editor_foreground: Some(color(palette.code_foreground)),
            syntax: SyntaxColors {
                keyword: token(s.keyword),
                boolean: token(s.keyword),
                preproc: token(s.keyword),
                number: token(s.number),
                constant: token(s.number),
                function: token(s.function),
                constructor: token(s.function),
                variable: token(s.variable),
                variable_special: token(s.variable_special),
                property: token(s.variable_special),
                type_: token(s.r#type),
                enum_: token(s.r#type),
                variant: token(s.r#type),
                comment: token(s.comment),
                comment_doc: token(s.comment),
                string: token(s.string),
                string_escape: token(s.string),
                string_regex: token(s.string),
                string_special: token(s.string),
                string_special_symbol: token(s.string),
                tag: token(s.tag),
                tag_doctype: token(s.tag),
                attribute: token(s.variable_special),
                operator: token(palette.code_foreground),
                punctuation: token(palette.code_foreground),
                punctuation_bracket: token(palette.code_foreground),
                punctuation_delimiter: token(palette.code_foreground),
                ..Default::default()
            },
            ..Default::default()
        },
    })
}

pub(crate) fn reading_style(
    entry: &Entry,
    dark: bool,
    scale: f32,
    mono: SharedString,
) -> TextViewStyle {
    let theme = &entry.theme;
    let p = theme.palette(dark);
    let t = &theme.typography;
    let l = &theme.layout;
    let code_font = t
        .code_fonts
        .first()
        .cloned()
        .map(SharedString::from)
        .unwrap_or(mono);
    let mut style = TextViewStyle {
        is_dark: dark,
        paragraph_gap: rems(t.paragraph_gap / crate::core::config::UI_FONT_SIZE_DEFAULT),
        highlight_theme: highlight(theme, dark, entry.revision),
        link_color: Some(color(p.link)),
        link_hover_color: Some(color(p.link_hover)),
        inline_code_color: Some(color(p.inline_code_foreground)),
        inline_code_background: Some(color(p.inline_code_background)),
        inline_code_font: Some(code_font.clone()),
        inline_code_fallbacks: Some(gpui::FontFallbacks::from_fonts(t.code_fonts.clone())),
        selection_color: Some(color(p.selection)),
        border_color: Some(color(p.border)),
        task_color: Some(color(p.accent)),
        task_foreground: Some(color(p.paper)),
        quote_link_color: Some(color(p.quote_link)),
        quote_code_color: Some(color(p.quote_code_foreground)),
        quote_code_background: Some(color(p.quote_code_background)),
        table_code_background: Some(color(p.table_code_background)),
        table_hover_background: Some(color(p.table_hover)),
        code_block: refinement(
            div()
                .p(px(l.code_padding * scale))
                .rounded(px(l.code_radius * scale))
                .bg(color(p.code_background))
                .text_color(color(p.code_foreground))
                .border_1()
                .border_color(color(p.code_border))
                .font_family(code_font)
                .text_size(px(t.code_size * scale)),
        ),
        blockquote: refinement(
            div()
                .bg(color(p.quote_background))
                .text_color(color(p.quote_foreground))
                .border_l(px(l.quote_border * scale))
                .border_color(color(p.quote_border))
                .rounded_r(px(l.quote_radius * scale))
                .p(px(l.quote_padding * scale)),
        ),
        alert: refinement(
            div()
                .bg(color(p.alert_background))
                .text_color(color(p.alert_foreground))
                .border_color(color(p.alert_border)),
        ),
        nested_blockquote: refinement(
            div()
                .bg(color(p.nested_quote_background))
                .border_color(color(p.nested_quote_border)),
        ),
        table_cell: refinement(div().p(px(l.table_padding * scale)).border_r_0()),
        table_header: refinement(
            div()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(color(p.heading)),
        ),
        list_item: refinement(div().pb(px(l.list_gap * scale))),
        ..Default::default()
    };
    style.table.overflow.x = Some(gpui::Overflow::Scroll);
    style.code_block.overflow.x = Some(gpui::Overflow::Scroll);
    style.code_block.text.font_fallbacks =
        Some(gpui::FontFallbacks::from_fonts(t.code_fonts.clone()));
    for level in 0..6 {
        style.headings[level] = refinement(
            div()
                .text_size(px(t.heading_sizes[level] * scale))
                .font_weight(FontWeight(t.heading_weights[level]))
                .line_height(relative(t.heading_line_height))
                .text_color(color(p.heading))
                .mt(px(t.heading_gap * scale))
                .pb(px(t.paragraph_gap * scale * 0.5))
                .when(level == 1, |el| {
                    el.border_b_1().border_color(color(p.border))
                }),
        );
    }
    style
}

/// The source buffer owns all editable state. This entity retains only the
/// derived document, selection, reading focus and scroll position.
pub(crate) struct MarkdownPreview {
    source: Entity<InputState>,
    host: SharedHost,
    path: PathBuf,
    app: gpui::WeakEntity<Tty7App>,
    images: HashMap<PathBuf, TextViewImageSource>,
    image_bytes: usize,
    pending_anchor: Option<String>,
    last_position: Option<(usize, Pixels)>,
    restore_position: Option<(usize, Pixels)>,
    pub(crate) text: Entity<TextViewState>,
    pub(crate) scroll: ScrollHandle,
    revision: u64,
    width: Pixels,
    style_key: Option<(String, u64, bool, u32, SharedString)>,
    style: TextViewStyle,
    _subscriptions: Vec<Subscription>,
}

impl MarkdownPreview {
    pub(crate) fn new(
        source: Entity<InputState>,
        host: SharedHost,
        path: PathBuf,
        app: gpui::WeakEntity<Tty7App>,
        cx: &mut Context<Self>,
    ) -> Self {
        let content = source.read(cx).text().to_string();
        let text = cx.new(|cx| TextViewState::markdown_with_extensions(&content, extensions(), cx));
        let subscriptions = vec![
            cx.observe_global::<Registry>(|_, cx| cx.notify()),
            cx.observe_global::<gpui_component::Theme>(|_, cx| cx.notify()),
        ];
        cx.on_release(|this, cx| {
            for image in this.images.values() {
                if let TextViewImageSource::Ready(source) = image {
                    source.remove_asset(cx);
                }
            }
        })
        .detach();
        Self {
            source,
            host,
            path,
            app,
            images: HashMap::new(),
            image_bytes: 0,
            pending_anchor: None,
            last_position: None,
            restore_position: None,
            text,
            scroll: ScrollHandle::new(),
            revision: 0,
            width: px(0.),
            style_key: None,
            style: TextViewStyle::default(),
            _subscriptions: subscriptions,
        }
    }

    pub(crate) fn sync(&mut self, revision: u64, cx: &mut Context<Self>) {
        if self.revision == revision {
            return;
        }
        self.revision = revision;
        self.last_position = None;
        self.restore_position = None;
        let content = self.source.read(cx).text().to_string();
        self.text
            .update(cx, |state, cx| state.set_text(&content, cx));
        cx.notify();
    }

    pub(crate) fn navigate_anchor(&mut self, anchor: String, cx: &mut Context<Self>) {
        self.pending_anchor = Some(anchor);
        cx.notify();
    }

    fn open_link(&mut self, target: &str, window: &mut Window, cx: &mut Context<Self>) {
        match markdown_document::resolve(&self.path, target, self.host.id().is_local()) {
            Ok(Target::Web(url)) => cx.open_url(&url),
            Ok(Target::Anchor(anchor)) => self.navigate_anchor(anchor, cx),
            Ok(Target::File { path, fragment }) => {
                let host = self.host.clone();
                let _ = self.app.update(cx, |app, cx| {
                    app.editor_open_markdown_link(host, &path, fragment, window, cx)
                });
            }
            Err(error) => window.push_notification(error, cx),
        }
    }

    fn image_source(
        &mut self,
        url: &gpui::SharedUri,
        cx: &mut Context<Self>,
    ) -> TextViewImageSource {
        let path =
            match markdown_document::resolve(&self.path, url.as_ref(), self.host.id().is_local()) {
                Ok(Target::Web(url)) => {
                    return TextViewImageSource::Ready(gpui::SharedUri::from(url).into());
                }
                Ok(Target::File { path, .. }) => path,
                Ok(Target::Anchor(_)) => {
                    return TextViewImageSource::Failed("image has no resource path".into());
                }
                Err(error) => return TextViewImageSource::Failed(error.into()),
            };
        if let Some(image) = self.images.get(&path) {
            return image.clone();
        }
        if self.images.len() >= 64 {
            return TextViewImageSource::Failed("document image limit reached (64)".into());
        }
        self.images
            .insert(path.clone(), TextViewImageSource::Loading);
        let requested = path.clone();
        HostOps::run(
            self.host.clone(),
            cx,
            move |host| {
                let metadata = host.stat(&requested).map_err(|e| e.to_string())?;
                if metadata.len > markdown_document::MAX_IMAGE_BYTES {
                    return Err("image exceeds 8 MiB".to_string());
                }
                let bytes = host
                    .read_file(&requested, markdown_document::MAX_IMAGE_BYTES)
                    .map_err(|e| e.to_string())?;
                let format = markdown_document::image_format(&bytes)?;
                Ok((format, bytes))
            },
            move |this, result, cx| {
                let image = match result {
                    Ok((format, bytes)) if this.image_bytes + bytes.len() <= 32 * 1024 * 1024 => {
                        this.image_bytes += bytes.len();
                        TextViewImageSource::Ready(gpui::ImageSource::Image(Arc::new(
                            gpui::Image::from_bytes(format, bytes),
                        )))
                    }
                    Ok(_) => {
                        TextViewImageSource::Failed("document image cache exceeds 32 MiB".into())
                    }
                    Err(error) => TextViewImageSource::Failed(error.into()),
                };
                this.images.insert(path, image);
                cx.notify();
            },
        );
        TextViewImageSource::Loading
    }

    fn on_key_down(&mut self, event: &gpui::KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let key = &event.keystroke;
        if key.modifiers.control || key.modifiers.platform || key.modifiers.alt {
            return;
        }
        let page = self.scroll.bounds().size.height * 0.9;
        let offset = self.scroll.offset();
        let y = match key.key.as_str() {
            "up" => offset.y + px(40.),
            "down" => offset.y - px(40.),
            "pageup" => offset.y + page,
            "pagedown" => offset.y - page,
            "space" if key.modifiers.shift => offset.y + page,
            "space" => offset.y - page,
            "home" => px(0.),
            "end" => -self.scroll.max_offset().y,
            _ => return,
        };
        self.scroll.set_offset(point(
            offset.x,
            y.clamp(-self.scroll.max_offset().y, px(0.)),
        ));
        cx.stop_propagation();
        cx.notify();
    }
}

impl Focusable for MarkdownPreview {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.text.read(cx).focus_handle(cx)
    }
}

impl Render for MarkdownPreview {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entry = current(cx);
        let dark = cx.theme().mode.is_dark();
        let scale = f32::from(window.rem_size()) / crate::core::config::UI_FONT_SIZE_DEFAULT;
        let mono = cx.theme().mono_font_family.clone();
        let key = (
            entry.theme.id.clone(),
            entry.revision,
            dark,
            scale.to_bits(),
            mono.clone(),
        );
        if self.style_key.as_ref() != Some(&key) {
            self.restore_position = self.last_position;
            self.style = reading_style(&entry, dark, scale, mono);
            self.style_key = Some(key);
        }
        let palette = entry.theme.palette(dark);
        let typography = &entry.theme.typography;
        let layout = &entry.theme.layout;
        let compact = self.width < px(layout.compact_below * scale);
        let padding = if compact {
            layout.compact_padding
        } else {
            layout.padding
        } * scale;
        let outer = if compact {
            0.
        } else {
            layout.outer_padding * scale
        };
        let body_font = typography
            .fonts
            .first()
            .cloned()
            .map(SharedString::from)
            .unwrap_or_else(|| cx.theme().font_family.clone());
        let copy_color = color(palette.muted);
        let link_owner = cx.weak_entity();
        let image_owner = cx.weak_entity();
        let text = TextView::new(&self.text)
            .markdown_extensions(extensions())
            .selectable(true)
            .style(self.style.clone())
            .on_link(move |url, window, cx| {
                let _ = link_owner.update(cx, |this, cx| this.open_link(url, window, cx));
            })
            .image_source(move |url, _, cx| {
                image_owner
                    .update(cx, |this, cx| this.image_source(url, cx))
                    .unwrap_or(TextViewImageSource::Failed("document closed".into()))
            })
            .code_block_actions(move |block, _, _| {
                let code = block.code();
                Button::new("copy-code")
                    .icon(IconName::Copy)
                    .ghost()
                    .xsmall()
                    .text_color(copy_color)
                    .tooltip(t(L10nKey::EditorCopyCode))
                    .on_click(move |_, _, cx| {
                        cx.write_to_clipboard(gpui::ClipboardItem::new_string(code.to_string()));
                    })
            });
        let mut card = div()
            .w_full()
            .max_w(px(layout.max_width * scale))
            .mx_auto()
            .p(px(padding))
            .text_size(px(typography.font_size * scale))
            .line_height(relative(typography.line_height))
            .font_family(body_font)
            .text_color(color(palette.foreground))
            .bg(color(palette.paper));
        card.style().text.font_fallbacks =
            Some(gpui::FontFallbacks::from_fonts(typography.fonts.clone()));
        let card = card
            .when(!compact, |el| {
                el.rounded(px(layout.radius * scale))
                    .border_1()
                    .border_color(color(palette.paper_border))
                    .shadow(vec![BoxShadow {
                        color: color(palette.shadow),
                        offset: point(px(0.), px(layout.shadow_offset * scale)),
                        blur_radius: px(layout.shadow_blur * scale),
                        spread_radius: px(0.),
                        inset: false,
                    }])
            })
            .child(text);
        let entity = cx.weak_entity();
        let scroll = self.scroll.clone();
        crate::ui::scrollbar::with_vertical_scrollbar(
            "markdown-reading-scrollbar",
            div()
                .id("markdown-reading")
                .size_full()
                .key_context("MarkdownPreview")
                .bg(color(palette.background))
                .overflow_y_scroll()
                .track_scroll(&scroll)
                .p(px(outer))
                .on_key_down(cx.listener(Self::on_key_down))
                .child(card)
                .on_prepaint(move |_, window, cx| {
                    let _ = entity.update(cx, |this, cx| {
                        // The helper canvas is a scrolling child. The handle
                        // supplies the fixed viewport in window coordinates.
                        let bounds = this.scroll.bounds();
                        if this.width != bounds.size.width {
                            this.width = bounds.size.width;
                            this.restore_position = this.restore_position.or(this.last_position);
                            cx.notify();
                            return;
                        }
                        if let Some(anchor) = this.pending_anchor.take() {
                            this.restore_position = None;
                            if anchor.is_empty() {
                                this.scroll.set_offset(point(px(0.), px(0.)));
                            } else if let Some(target) = this.text.read(cx).anchor_bounds(&anchor) {
                                let offset = this.scroll.offset();
                                let y = offset.y - (target.top() - bounds.top()) + px(12.);
                                this.scroll.set_offset(point(
                                    offset.x,
                                    y.clamp(-this.scroll.max_offset().y, px(0.)),
                                ));
                            } else {
                                window.push_notification(
                                    format!("#{anchor}: {}", t(L10nKey::MarkdownAnchorMissing)),
                                    cx,
                                );
                            }
                            cx.notify();
                            return;
                        }
                        if let Some((index, inside)) = this.restore_position.take() {
                            if let Some(target) = this.text.read(cx).block_bounds(index) {
                                let offset = this.scroll.offset();
                                let y = offset.y + bounds.top() - inside - target.top();
                                let y = y.clamp(-this.scroll.max_offset().y, px(0.));
                                if (y - offset.y).abs() > px(0.1) {
                                    this.scroll.set_offset(point(offset.x, y));
                                    cx.notify();
                                    return;
                                }
                            }
                        }
                        this.last_position = this.text.read(cx).reading_position(bounds.top());
                    });
                }),
            &scroll,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{TestAppContext, VisualTestContext};
    use std::{
        io,
        path::Path,
        sync::{
            Mutex,
            atomic::{AtomicBool, Ordering},
            mpsc,
        },
        time::{Duration, Instant},
    };
    use tty7_core::host::{self, Host, HostId, Meta};

    // Only the Host I/O boundary is substituted. The editor, resource adapter,
    // renderer and async completion all run through their application paths.
    struct DocumentsHost {
        id: HostId,
        files: HashMap<PathBuf, Vec<u8>>,
        reads: Mutex<Vec<PathBuf>>,
        image_gate: Mutex<Option<mpsc::Receiver<()>>>,
        image_completed: AtomicBool,
    }

    impl DocumentsHost {
        fn new(id: u64, label: &str, gate: Option<mpsc::Receiver<()>>) -> Arc<Self> {
            Arc::new(Self {
                id: HostId(id),
                files: HashMap::from([
                    ("/repo/docs/readme.md".into(), format!("# {label}\n\n![{label}](images/icon.svg)").into_bytes()),
                    ("/repo/docs/images/icon.svg".into(), format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"16\" height=\"16\"><title>{label}</title><rect width=\"16\" height=\"16\" fill=\"blue\"/></svg>").into_bytes()),
                ]),
                reads: Mutex::default(), image_gate: Mutex::new(gate), image_completed: AtomicBool::new(false),
            })
        }
    }

    fn unsupported<T>() -> io::Result<T> {
        Err(io::ErrorKind::Unsupported.into())
    }

    impl Host for DocumentsHost {
        fn id(&self) -> HostId {
            self.id
        }
        fn separator(&self) -> char {
            '/'
        }
        fn is_absolute(&self, path: &Path) -> bool {
            path.to_string_lossy().starts_with('/')
        }
        fn read_dir(&self, _: &Path, _: Option<&Path>) -> io::Result<Vec<host::Entry>> {
            Ok(Vec::new())
        }
        fn stat(&self, path: &Path) -> io::Result<Meta> {
            let bytes = self.files.get(path).ok_or(io::ErrorKind::NotFound)?;
            Ok(Meta {
                is_dir: false,
                is_symlink: false,
                len: bytes.len() as u64,
                mtime: None,
                readonly: false,
            })
        }
        fn read_file(&self, path: &Path, max: u64) -> io::Result<Vec<u8>> {
            self.reads.lock().unwrap().push(path.to_owned());
            let bytes = self.files.get(path).ok_or(io::ErrorKind::NotFound)?;
            if bytes.len() as u64 > max {
                return Err(io::ErrorKind::InvalidData.into());
            }
            if path.extension().is_some_and(|ext| ext == "svg") {
                if let Some(gate) = self.image_gate.lock().unwrap().take() {
                    gate.recv_timeout(Duration::from_secs(10))
                        .map_err(|_| io::ErrorKind::TimedOut)?;
                }
                self.image_completed.store(true, Ordering::SeqCst);
            }
            Ok(bytes.clone())
        }
        fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
            Ok(path.to_owned())
        }
        fn search(
            &self,
            _: &[PathBuf],
            _: &str,
            _: usize,
            _: usize,
            _: bool,
        ) -> io::Result<Vec<host::SearchHit>> {
            Ok(Vec::new())
        }
        fn write_file(&self, _: &Path, _: &[u8]) -> io::Result<Meta> {
            unsupported()
        }
        fn create_file_new(&self, _: &Path) -> io::Result<()> {
            unsupported()
        }
        fn create_dir(&self, _: &Path, _: bool) -> io::Result<()> {
            unsupported()
        }
        fn rename(&self, _: &Path, _: &Path) -> io::Result<()> {
            unsupported()
        }
        fn remove(&self, _: &Path, _: bool) -> io::Result<()> {
            unsupported()
        }
        fn repo_root(&self, _: &Path) -> io::Result<Option<PathBuf>> {
            Ok(None)
        }
        fn git(&self, _: &Path, _: &[&str]) -> io::Result<host::Output> {
            unsupported()
        }
        fn shells(&self) -> io::Result<host::ShellInventory> {
            unsupported()
        }
        fn watch(&self, _: &[PathBuf]) -> io::Result<host::WatchSub> {
            unsupported()
        }
    }

    #[track_caller]
    fn settle(vcx: &mut VisualTestContext, mut ready: impl FnMut(&mut VisualTestContext) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            vcx.run_until_parked();
            vcx.update(|window, cx| {
                let _ = window.draw(cx);
            });
            if ready(vcx) {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "document resource operation did not settle"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[gpui::test]
    fn markdown_resources_use_the_owning_host_and_ignore_closed_document_completions(
        cx: &mut TestAppContext,
    ) {
        let (app, mut vcx) = crate::ui::app::test_window::harness(cx);
        app.update_in(&mut vcx, |app, _, cx| {
            init(Default::default(), cx);
            app.tabs
                .push(crate::ui::app::Tab::new(crate::ui::pane::Pane::Empty));
            app.active = app.tabs.len() - 1;
        });
        let (release, gate) = mpsc::channel();
        let first = DocumentsHost::new(901, "First host", Some(gate));
        let second = DocumentsHost::new(902, "Second host", None);
        let document = Path::new("/repo/docs/readme.md");
        let image = Path::new("/repo/docs/images/icon.svg");
        app.update_in(&mut vcx, |app, window, cx| {
            app.editor_open_on_host(first.clone(), document, window, cx)
        });
        settle(&mut vcx, |_| {
            first.reads.lock().unwrap().iter().any(|path| path == image)
        });
        let closed = app.read_with(&vcx, |app, _| {
            app.tab_code()
                .unwrap()
                .active_file()
                .unwrap()
                .reading
                .as_ref()
                .unwrap()
                .downgrade()
        });

        app.update_in(&mut vcx, |app, window, cx| {
            app.editor_open_on_host(second.clone(), document, window, cx)
        });
        settle(&mut vcx, |cx| {
            app.read_with(cx, |app, _| app.tab_code().unwrap().files.len() == 2)
        });
        let reading = app.read_with(&vcx, |app, _| {
            app.tab_code()
                .unwrap()
                .active_file()
                .unwrap()
                .reading
                .clone()
                .unwrap()
        });
        settle(&mut vcx, |cx| {
            reading.read_with(cx, |reading, _| {
                matches!(
                    reading.images.get(image),
                    Some(TextViewImageSource::Ready(_))
                )
            })
        });
        reading.read_with(&vcx, |reading, cx| {
            assert!(reading.text.read(cx).source().contains("Second host"))
        });

        app.update_in(&mut vcx, |app, window, cx| {
            let index = app
                .tab_code()
                .unwrap()
                .files
                .iter()
                .position(|file| file.host.id() == first.id())
                .unwrap();
            app.editor_close_file(index, window, cx)
        });
        settle(&mut vcx, |_| closed.upgrade().is_none());
        release.send(()).unwrap();
        settle(&mut vcx, |_| first.image_completed.load(Ordering::SeqCst));
        vcx.run_until_parked();
        reading.read_with(&vcx, |reading, _| {
            let Some(TextViewImageSource::Ready(gpui::ImageSource::Image(data))) =
                reading.images.get(image)
            else {
                panic!("loaded host image");
            };
            assert!(String::from_utf8_lossy(&data.bytes).contains("Second host"));
            assert_eq!(reading.host.id(), second.id());
        });
        assert_eq!(
            second
                .reads
                .lock()
                .unwrap()
                .iter()
                .filter(|path| *path == image)
                .count(),
            1
        );
    }
}

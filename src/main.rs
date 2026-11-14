#![windows_subsystem = "windows"]

mod bangs;
mod config;
mod hotkey;
mod index;
mod logger;
mod note;
mod search;
mod settings;
mod sources;
mod web;
mod win;

use eframe::egui;
use std::sync::{Arc, Mutex};

#[derive(PartialEq)]
enum View {
    Palette,
    Note,
    Settings,
}

pub struct App {
    cfg: config::Config,
    catalog: Vec<sources::Item>,
    files: Arc<index::FileIndex>,
    web: web::WebHandle,

    prefix: String,
    suffix: String,
    sel: usize,
    view: View,
    note: Option<note::NoteState>,
    settings: settings::SettingsState,

    hotkey_rx: Arc<Mutex<Vec<hotkey::HotkeyEvent>>>,
    safe_mode: bool,
    last_h: f32,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>, safe_mode: bool) -> Self {
        let cfg = config::Config::load();
        logger::info("palette starting");

        let files = Arc::new(index::FileIndex::new());
        if !safe_mode {
            index::spawn_indexer(cfg.roots.clone(), cfg.excludes.clone(), files.clone());
        }

        let catalog = sources::build(&cfg);
        let web = web::WebHandle::new(safe_mode);

        hotkey::install_context(cc.egui_ctx.clone());

        Self {
            cfg,
            catalog,
            files,
            web,
            prefix: String::new(),
            suffix: String::new(),
            sel: 0,
            view: View::Palette,
            note: None,
            settings: settings::SettingsState::default(),
            hotkey_rx: hotkey::events(),
            safe_mode,
            last_h: 0.0,
        }
    }

    fn raw(&self) -> String {
        format!("{}{}", self.prefix, self.suffix)
    }

    fn chip(&self) -> Option<bangs::ChipInfo> {
        if self.prefix.is_empty() {
            return None;
        }
        bangs::parse(&self.prefix, &self.cfg.bangs).map(|(c, _)| c)
    }

    fn promote_chip(&mut self) {
        if !self.prefix.is_empty() {
            return;
        }
        if let Some((_info, off)) = bangs::parse(&self.suffix, &self.cfg.bangs) {
            self.prefix = self.suffix[..off].to_string();
            self.suffix = self.suffix[off..].to_string();
            self.sel = 0;
        }
    }

    fn clear_chip(&mut self) {
        self.prefix.clear();
    }

    fn show_palette(&mut self, ctx: &egui::Context, prefill_web: bool) {
        if prefill_web && !self.safe_mode {
            self.prefix = "w ".into();
            self.suffix.clear();
            self.sel = 0;
        }
        self.view = View::Palette;
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(680.0, 400.0)));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    }

    fn hide(&mut self, ctx: &egui::Context) {
        if self.view == View::Note {
            self.save_note();
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
    }

    fn save_note(&mut self) {
        if let Some(n) = &mut self.note {
            if let Err(e) = n.save() {
                logger::warn(&format!("note save failed: {e}"));
            }
        }
    }

    fn drain_hotkeys(&mut self, ctx: &egui::Context) {
        let events: Vec<_> = {
            let mut g = self.hotkey_rx.lock().unwrap();
            g.drain(..).collect()
        };
        for ev in events {
            let focused = ctx.input(|i| i.viewport().focused.unwrap_or(false));
            if focused {
                continue;
            }
            if ev.browser_focused {
                self.show_palette(ctx, true);
            } else {
                self.show_palette(ctx, false);
            }
        }
    }

    fn execute(&mut self, ctx: &egui::Context) {
        let raw = self.raw();
        let chip = self.chip();
        let query = self.suffix.clone();

        if let Some(c) = chip {
            match c.kind {
                bangs::ChipKind::Web => {
                    if query.trim().is_empty() {
                        return;
                    }
                    let url = web::resolve(&query);
                    self.web.navigate(url);
                    self.hide(ctx);
                    return;
                }
                bangs::ChipKind::Note => {
                    if !self.safe_mode {
                        self.note = Some(note::NoteState::open(&self.cfg, &query));
                        self.view = View::Note;
                        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(
                            egui::vec2(720.0, 720.0),
                        ));
                    }
                    return;
                }
                bangs::ChipKind::Settings => {
                    self.open_settings(ctx);
                    return;
                }
                bangs::ChipKind::Bang(i) => {
                    if query.trim().is_empty() {
                        return;
                    }
                    let b = &self.cfg.bangs[i];
                    let url = b.url.replace("{query}", &urlencode(&query));
                    win::shell_open(&url);
                    self.hide(ctx);
                    return;
                }
            }
        }

        let results = self.results(&raw);
        if let Some(&(_, idx)) = results.get(self.sel) {
            let action = self.catalog[idx].action.clone();
            self.run_action(ctx, action);
        } else if !raw.trim().is_empty() {
            let url = format!(
                "https://www.google.com/search?q={}",
                urlencode(raw.trim())
            );
            win::shell_open(&url);
            self.hide(ctx);
        }
    }

    fn run_action(&mut self, ctx: &egui::Context, a: sources::Action) {
        match a {
            sources::Action::Open(path) => { win::shell_open(&path); }
            sources::Action::MsSettings(uri) => { win::shell_open(&uri); }
            sources::Action::Run { exe, args, cwd } => { win::spawn(&exe, &args, &cwd); }
            sources::Action::Internal(name) => match name {
                "settings" => { self.open_settings(ctx); return; }
                "quit" => {
                    self.save_note();
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    return;
                }
                "reload" => {
                    self.catalog = sources::build(&self.cfg);
                    index::spawn_indexer(
                        self.cfg.roots.clone(),
                        self.cfg.excludes.clone(),
                        self.files.clone(),
                    );
                }
                _ => {}
            },
        }
        self.hide(ctx);
    }

    fn results(&self, raw: &str) -> Vec<(u32, usize)> {
        let q = if self.prefix.is_empty() {
            raw.trim().to_string()
        } else {
            self.suffix.trim().to_string()
        };
        if q.is_empty() {
            return Vec::new();
        }
        let mut items = self.catalog.clone();
        let files = self.files.snapshot();
        for p in files.iter().take(50_000) {
            if let Some(name) = p.file_name().and_then(|s| s.to_str()) {
                items.push(sources::Item {
                    title: name.to_string(),
                    subtitle: p.to_string_lossy().into_owned(),
                    cat: sources::Category::Files,
                    action: sources::Action::Open(p.to_string_lossy().into_owned()),
                    haystack: format!("{} {}", name, p.to_string_lossy()),
                });
            }
        }
        search::rank(&q, &items, self.cfg.max_results)
    }

    fn open_settings(&mut self, ctx: &egui::Context) {
        self.settings = settings::SettingsState::default();
        self.view = View::Settings;
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(880.0, 620.0)));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    }

    fn ui_settings(&mut self, ctx: &egui::Context) {
        let mut close = false;
        let mut quit = false;

        egui::CentralPanel::default().show(ctx, |ui| {
            let out = settings::show(ui, &mut self.settings, &mut self.cfg);
            if out.catalog_stale {
                self.catalog = sources::build(&self.cfg);
            }
            close = out.close;
            quit = out.quit;
        });

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            close = true;
        }

        if close {
            self.catalog = sources::build(&self.cfg);
            self.prefix.clear();
            self.suffix.clear();
            self.sel = 0;
            self.view = View::Palette;
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(680.0, 400.0)));
            if quit {
                self.save_note();
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            }
        }
    }
}

fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_hotkeys(ctx);
        self.promote_chip();

        let focused = ctx.input(|i| i.viewport().focused.unwrap_or(false));
        if !focused && self.view == View::Palette {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }

        match self.view {
            View::Palette => self.ui_palette(ctx),
            View::Note => self.ui_note(ctx),
            View::Settings => self.ui_settings(ctx),
        }
    }
}

impl App {
    fn ui_palette(&mut self, ctx: &egui::Context) {
        let theme = match self.cfg.theme {
            config::Theme::Dark => egui::Theme::Dark,
            config::Theme::Light => egui::Theme::Light,
            config::Theme::System => egui::Theme::Dark,
        };
        ctx.set_theme(theme);
        ctx.set_pixels_per_point(self.cfg.font_scale);

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(ctx.style().visuals.window_fill))
            .show(ctx, |ui| {
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.add_space(12.0);
                    if let Some(chip) = self.chip() {
                        let del = egui::Frame::none()
                            .fill(ui.visuals().widgets.active.bg_fill)
                            .rounding(10.0)
                            .inner_margin(egui::Margin::symmetric(8.0, 3.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(&chip.label)
                                            .color(ui.visuals().selection.bg_fill)
                                            .strong(),
                                    );
                                    ui.add_space(4.0);
                                    if ui
                                        .add(egui::Button::new("×").small().frame(false))
                                        .clicked()
                                    {
                                        self.clear_chip();
                                    }
                                });
                            })
                            .response;
                        if del.clicked() {
                            ui.memory_mut(|m| m.request_focus(egui::Id::new("palette_input")));
                        }
                        ui.add_space(4.0);
                    }

                    let out = egui::TextEdit::singleline(&mut self.suffix)
                        .id(egui::Id::new("palette_input"))
                        .frame(false)
                        .desired_width(f32::INFINITY)
                        .hint_text("Search apps, files, commands…")
                        .font(egui::FontId::proportional(16.0))
                        .show(ui);

                    let caret_at_zero = out
                        .state
                        .cursor
                        .char_range()
                        .map(|r| r.primary.index == 0)
                        .unwrap_or(false);

                    if !self.prefix.is_empty()
                        && out.response.has_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Backspace))
                        && caret_at_zero
                    {
                        self.clear_chip();
                    }

                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        self.hide(ctx);
                    }
                    if out.response.lost_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter))
                    {
                        self.execute(ctx);
                    }
                });
                ui.add_space(8.0);
                ui.separator();

                let raw = self.raw();
                let results = self.results(&raw);
                if self.sel >= results.len() {
                    self.sel = 0;
                }

                egui::ScrollArea::vertical().max_height(340.0).show(ui, |ui| {
                    let mut last_cat: Option<sources::Category> = None;
                    for (i, (_score, idx)) in results.iter().enumerate() {
                        let item = &self.catalog[*idx];
                        if last_cat != Some(item.cat) {
                            last_cat = Some(item.cat);
                            ui.add_space(6.0);
                            ui.label(
                                egui::RichText::new(item.cat.label().to_uppercase())
                                    .size(11.0)
                                    .color(ui.visuals().weak_text_color()),
                            );
                        }
                        let selected = i == self.sel;
                        let resp = ui
                            .selectable_label(
                                selected,
                                egui::RichText::new(&item.title).size(14.0),
                            )
                            .on_hover_text(&item.subtitle);
                        if resp.clicked() {
                            self.sel = i;
                            self.execute(ctx);
                        }
                        if selected && ui.input(|k| k.key_pressed(egui::Key::Enter)) {
                            self.execute(ctx);
                        }
                    }
                });

                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("↑↓ navigate · Enter run · Esc close · Ctrl+H hide")
                                .size(11.0)
                                .color(ui.visuals().weak_text_color()),
                        );
                    });
                });
            });

        let results = self.results(&self.raw());
        let desired = (72.0 + results.len().min(12) as f32 * 34.0).clamp(120.0, 620.0);
        if (desired - self.last_h).abs() > 1.0 {
            self.last_h = desired;
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(680.0, desired)));
        }
    }

    fn ui_note(&mut self, ctx: &egui::Context) {
        let Some(state) = &mut self.note else {
            self.view = View::Palette;
            return;
        };
        if ui_note_inner(ctx, state) {
            self.save_note();
            self.note = None;
            self.view = View::Palette;
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(680.0, 400.0)));
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
    }
}

fn ui_note_inner(ctx: &egui::Context, state: &mut note::NoteState) -> bool {
    let mut close = false;
    egui::TopBottomPanel::bottom("note_status").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.label(format!("{} words", state.word_count()));
        });
    });
    egui::CentralPanel::default().show(ctx, |ui| {
        if note::editor(ui, state) {
            close = true;
        }
    });
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        close = true;
    }
    close
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let safe_mode = args.iter().any(|a| a == "--safe-mode");
    logger::init();

    if !win::single_instance() {
        win::focus_existing();
        return;
    }

    hotkey::spawn();

    let viewport = egui::ViewportBuilder::default()
        .with_title("Palette")
        .with_decorations(false)
        .with_always_on_top()
        .with_transparent(false)
        .with_visible(false)
        .with_resizable(true)
        .with_taskbar(false)
        .with_inner_size([680.0, 400.0]);

    let opts = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    let r = eframe::run_native(
        "Palette",
        opts,
        Box::new(move |cc| Ok(Box::new(App::new(cc, safe_mode)))),
    );
    if let Err(e) = r {
        logger::warn(&format!("eframe exited: {e}"));
    }
}

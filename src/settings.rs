use crate::config::{Bang, Bookmark, Config, Theme, UserCommand};
use crate::win;
use eframe::egui;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Section {
    General,
    Appearance,
    Bangs,
    Commands,
    QuickFolders,
    FileSearch,
    Bookmarks,
    Notes,
    Blocklist,
    About,
}

impl Section {
    pub fn all() -> &'static [Section] {
        &[
            Section::General,
            Section::Appearance,
            Section::Bangs,
            Section::Commands,
            Section::QuickFolders,
            Section::FileSearch,
            Section::Bookmarks,
            Section::Notes,
            Section::Blocklist,
            Section::About,
        ]
    }
    pub fn label(self) -> &'static str {
        match self {
            Section::General => "General",
            Section::Appearance => "Appearance",
            Section::Bangs => "Bangs",
            Section::Commands => "Commands",
            Section::QuickFolders => "Quick folders",
            Section::FileSearch => "File search",
            Section::Bookmarks => "Bookmarks",
            Section::Notes => "Notes",
            Section::Blocklist => "Blocklist",
            Section::About => "About",
        }
    }
}

const BUILTIN_COMMANDS: &[&str] = &[
    "Settings",
    "Quit Palette",
    "Reload index",
    "Task Manager",
    "Control Panel",
    "Terminal",
    "PowerShell",
    "Command Prompt",
    "Run dialog",
    "Lock",
    "Sleep",
    "Restart",
    "Shutdown",
    "Sign out",
];

#[derive(Default)]
pub struct SettingsState {
    pub section: Option<Section>,
    pub search: String,
    pub new_bang_trigger: String,
    pub new_bang_name: String,
    pub new_bang_url: String,
    pub new_bookmark_title: String,
    pub new_bookmark_url: String,
    pub new_blocklist: String,
    pub new_root: String,
    pub new_exclude: String,
    pub new_folder: String,
    pub new_cmd_name: String,
    pub new_cmd_exe: String,
    pub new_cmd_args: String,
}

pub struct Outcome {
    pub close: bool,
    pub quit: bool,
    pub catalog_stale: bool,
}

pub fn show(ui: &mut egui::Ui, state: &mut SettingsState, cfg: &mut Config) -> Outcome {
    let mut out = Outcome { close: false, quit: false, catalog_stale: false };

    egui::SidePanel::left("settings_nav")
        .resizable(false)
        .exact_width(170.0)
        .show_inside(ui, |ui| {
            ui.add_space(8.0);
            ui.add(
                egui::TextEdit::singleline(&mut state.search)
                    .hint_text("Filter…")
                    .desired_width(f32::INFINITY),
            );
            ui.add_space(8.0);
            let f = state.search.to_lowercase();
            for s in Section::all() {
                if !f.is_empty() && !s.label().to_lowercase().contains(&f) {
                    continue;
                }
                let selected = state.section == Some(*s);
                if ui.selectable_label(selected, s.label()).clicked() {
                    state.section = Some(*s);
                    state.search.clear();
                }
            }
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.add_space(8.0);
                if ui.button("Close").clicked() {
                    out.close = true;
                }
            });
        });

    egui::CentralPanel::default().show_inside(ui, |ui| {
        ui.add_space(8.0);
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| match state.section.unwrap_or(Section::General) {
                Section::General => general(ui, cfg, &mut out),
                Section::Appearance => appearance(ui, cfg, &mut out),
                Section::Bangs => bangs(ui, cfg, state, &mut out),
                Section::Commands => commands(ui, cfg, state, &mut out),
                Section::QuickFolders => quick_folders(ui, cfg, state, &mut out),
                Section::FileSearch => file_search(ui, cfg, state, &mut out),
                Section::Bookmarks => bookmarks(ui, cfg, state, &mut out),
                Section::Notes => notes(ui, cfg, &mut out),
                Section::Blocklist => blocklist(ui, cfg, state, &mut out),
                Section::About => about(ui, &mut out),
            });
    });

    out
}

fn heading(ui: &mut egui::Ui, text: &str, hint: &str) {
    ui.heading(text);
    if !hint.is_empty() {
        ui.label(egui::RichText::new(hint).weak());
    }
    ui.add_space(8.0);
}

fn commit(cfg: &Config, out: &mut Outcome) {
    if let Err(e) = cfg.save() {
        crate::logger::warn(&format!("config save failed: {e}"));
    }
    out.catalog_stale = true;
}

fn general(ui: &mut egui::Ui, cfg: &mut Config, out: &mut Outcome) {
    heading(ui, "General", "Global behaviour. Hotkey changes require a restart.");

    if crate::hotkey::did_fallback() {
        ui.colored_label(
            egui::Color32::from_rgb(220, 180, 60),
            "⚠ Ctrl+Space could not be registered (likely an IME conflict). \
             Using Ctrl+Shift+Space instead.",
        );
        ui.add_space(6.0);
    }

    ui.label("Hotkey");
    let mut changed = false;
    ui.horizontal(|ui| {
        changed |= ui.checkbox(&mut cfg.hotkey.ctrl, "Ctrl").changed();
        changed |= ui.checkbox(&mut cfg.hotkey.shift, "Shift").changed();
        changed |= ui.checkbox(&mut cfg.hotkey.alt, "Alt").changed();
        ui.label("+ Space");
    });
    if changed {
        cfg.hotkey.vk = 0x20;
        commit(cfg, out);
    }
    ui.add_space(10.0);

    if ui.checkbox(&mut cfg.open_at_cursor, "Open at mouse cursor").changed() {
        commit(cfg, out);
    }
    if ui.checkbox(&mut cfg.compact, "Compact mode").changed() {
        commit(cfg, out);
    }
    if ui
        .checkbox(&mut cfg.show_suggestions_when_empty, "Show suggestions when empty")
        .changed()
    {
        commit(cfg, out);
    }
    if ui
        .checkbox(&mut cfg.free_browser_memory, "Free browser memory when closed")
        .changed()
    {
        commit(cfg, out);
    }
    if ui
        .checkbox(&mut cfg.low_memory_mode, "Low memory mode (disables GPU)")
        .changed()
    {
        commit(cfg, out);
    }

    ui.add_space(10.0);
    ui.label("Max results per category");
    if ui.add(egui::Slider::new(&mut cfg.max_results, 5..=50)).changed() {
        commit(cfg, out);
    }
}

fn appearance(ui: &mut egui::Ui, cfg: &mut Config, out: &mut Outcome) {
    heading(ui, "Appearance", "Theme, density, and scale.");

    ui.label("Theme");
    ui.horizontal(|ui| {
        for (t, l) in [
            (Theme::Light, "Light"),
            (Theme::Dark, "Dark"),
            (Theme::System, "System"),
        ] {
            if ui.radio(cfg.theme == t, l).clicked() {
                cfg.theme = t;
                commit(cfg, out);
            }
        }
    });
    ui.add_space(10.0);

    ui.label("Font scale");
    ui.horizontal(|ui| {
        for s in [0.9_f32, 1.0, 1.1, 1.25] {
            if ui
                .radio(
                    (cfg.font_scale - s).abs() < 0.01,
                    format!("{}%", (s * 100.0) as i32),
                )
                .clicked()
            {
                cfg.font_scale = s;
                commit(cfg, out);
            }
        }
    });
    ui.add_space(10.0);

    if ui.checkbox(&mut cfg.compact, "Compact row density").changed() {
        commit(cfg, out);
    }
}

fn bangs(ui: &mut egui::Ui, cfg: &mut Config, state: &mut SettingsState, out: &mut Outcome) {
    heading(
        ui,
        "Bangs",
        "Trigger must be a bare word (no !, no spaces). Reserved: w, wb, note, settings.",
    );

    let mut remove: Option<usize> = None;
    let mut changed = false;
    for (i, b) in cfg.bangs.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            changed |= ui
                .add(egui::TextEdit::singleline(&mut b.trigger).desired_width(60.0))
                .changed();
            changed |= ui
                .add(egui::TextEdit::singleline(&mut b.name).desired_width(110.0))
                .changed();
            changed |= ui
                .add(egui::TextEdit::singleline(&mut b.url).desired_width(360.0))
                .changed();
            if ui.button("×").clicked() {
                remove = Some(i);
            }
        });
    }
    if changed {
        commit(cfg, out);
    }
    if let Some(i) = remove {
        cfg.bangs.remove(i);
        commit(cfg, out);
    }

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label("Add");
        ui.add(
            egui::TextEdit::singleline(&mut state.new_bang_trigger)
                .hint_text("trigger")
                .desired_width(60.0),
        );
        ui.add(
            egui::TextEdit::singleline(&mut state.new_bang_name)
                .hint_text("Name")
                .desired_width(110.0),
        );
        ui.add(
            egui::TextEdit::singleline(&mut state.new_bang_url)
                .hint_text("https://…{query}")
                .desired_width(360.0),
        );

        let t = state.new_bang_trigger.trim();
        let valid = !t.is_empty()
            && !t.contains(char::is_whitespace)
            && !t.starts_with('!')
            && !crate::bangs::is_reserved(t)
            && !cfg.bangs.iter().any(|b| b.trigger.eq_ignore_ascii_case(t));
        if ui.add_enabled(valid, egui::Button::new("Add")).clicked() {
            cfg.bangs.push(Bang {
                trigger: t.to_string(),
                name: state.new_bang_name.trim().to_string(),
                url: state.new_bang_url.trim().to_string(),
                keywords: Vec::new(),
            });
            state.new_bang_trigger.clear();
            state.new_bang_name.clear();
            state.new_bang_url.clear();
            commit(cfg, out);
        }
    });

    ui.add_space(12.0);
    if ui.button("Reset bangs to defaults").clicked() {
        cfg.bangs = Config::default().bangs;
        commit(cfg, out);
    }
}

fn commands(ui: &mut egui::Ui, cfg: &mut Config, state: &mut SettingsState, out: &mut Outcome) {
    heading(ui, "Commands", "Built-in toggles and user-defined launchers.");

    ui.label("Built-in");
    for name in BUILTIN_COMMANDS {
        let mut on = !cfg.disabled_builtins.iter().any(|d| d == name);
        if ui.checkbox(&mut on, *name).changed() {
            cfg.disabled_builtins.retain(|d| d != name);
            if !on {
                cfg.disabled_builtins.push((*name).to_string());
            }
            commit(cfg, out);
        }
    }

    ui.add_space(12.0);
    ui.label("User commands");
    let mut remove: Option<usize> = None;
    let mut changed = false;
    for (i, c) in cfg.commands.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            changed |= ui
                .add(egui::TextEdit::singleline(&mut c.name).desired_width(140.0))
                .changed();
            changed |= ui
                .add(
                    egui::TextEdit::singleline(&mut c.exe)
                        .hint_text("exe")
                        .desired_width(220.0),
                )
                .changed();
            changed |= ui
                .add(
                    egui::TextEdit::singleline(&mut c.args)
                        .hint_text("args")
                        .desired_width(160.0),
                )
                .changed();
            if ui.button("×").clicked() {
                remove = Some(i);
            }
        });
    }
    if changed {
        commit(cfg, out);
    }
    if let Some(i) = remove {
        cfg.commands.remove(i);
        commit(cfg, out);
    }

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut state.new_cmd_name)
                .hint_text("Name")
                .desired_width(140.0),
        );
        ui.add(
            egui::TextEdit::singleline(&mut state.new_cmd_exe)
                .hint_text("C:\\path\\to.exe")
                .desired_width(220.0),
        );
        ui.add(
            egui::TextEdit::singleline(&mut state.new_cmd_args)
                .hint_text("args")
                .desired_width(160.0),
        );
        let valid = !state.new_cmd_name.trim().is_empty()
            && !state.new_cmd_exe.trim().is_empty();
        if ui.add_enabled(valid, egui::Button::new("Add")).clicked() {
            cfg.commands.push(UserCommand {
                name: state.new_cmd_name.trim().to_string(),
                exe: state.new_cmd_exe.trim().to_string(),
                args: state.new_cmd_args.trim().to_string(),
                cwd: String::new(),
                keywords: Vec::new(),
                terminal: false,
            });
            state.new_cmd_name.clear();
            state.new_cmd_exe.clear();
            state.new_cmd_args.clear();
            commit(cfg, out);
        }
    });
}

fn quick_folders(
    ui: &mut egui::Ui,
    cfg: &mut Config,
    state: &mut SettingsState,
    out: &mut Outcome,
) {
    heading(ui, "Quick folders", "Shown as their own category.");

    let mut remove: Option<usize> = None;
    let mut changed = false;
    for (i, p) in cfg.quick_folders.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            let mut s = p.to_string_lossy().into_owned();
            if ui
                .add(egui::TextEdit::singleline(&mut s).desired_width(440.0))
                .changed()
            {
                *p = std::path::PathBuf::from(s);
                changed = true;
            }
            if ui.button("×").clicked() {
                remove = Some(i);
            }
        });
    }
    if changed {
        commit(cfg, out);
    }
    if let Some(i) = remove {
        cfg.quick_folders.remove(i);
        commit(cfg, out);
    }

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut state.new_folder)
                .hint_text("C:\\Folder")
                .desired_width(440.0),
        );
        if ui
            .add_enabled(
                !state.new_folder.trim().is_empty(),
                egui::Button::new("Add"),
            )
            .clicked()
        {
            cfg.quick_folders
                .push(std::path::PathBuf::from(state.new_folder.trim()));
            state.new_folder.clear();
            commit(cfg, out);
        }
    });
}

fn file_search(
    ui: &mut egui::Ui,
    cfg: &mut Config,
    state: &mut SettingsState,
    out: &mut Outcome,
) {
    heading(ui, "File search", "Only these roots are indexed.");

    ui.label("Roots");
    let mut remove: Option<usize> = None;
    let mut changed = false;
    for (i, p) in cfg.roots.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            let mut s = p.to_string_lossy().into_owned();
            if ui
                .add(egui::TextEdit::singleline(&mut s).desired_width(420.0))
                .changed()
            {
                *p = std::path::PathBuf::from(s);
                changed = true;
            }
            if ui.button("×").clicked() {
                remove = Some(i);
            }
        });
    }
    if changed {
        commit(cfg, out);
    }
    if let Some(i) = remove {
        cfg.roots.remove(i);
        commit(cfg, out);
    }

    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut state.new_root)
                .hint_text("C:\\Folder")
                .desired_width(420.0),
        );
        if ui
            .add_enabled(!state.new_root.trim().is_empty(), egui::Button::new("Add root"))
            .clicked()
        {
            cfg.roots
                .push(std::path::PathBuf::from(state.new_root.trim()));
            state.new_root.clear();
            commit(cfg, out);
        }
    });

    ui.add_space(12.0);
    ui.label("Excludes (matched by folder name)");
    let mut remove: Option<usize> = None;
    let mut changed = false;
    for (i, e) in cfg.excludes.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            changed |= ui
                .add(egui::TextEdit::singleline(e).desired_width(300.0))
                .changed();
            if ui.button("×").clicked() {
                remove = Some(i);
            }
        });
    }
    if changed {
        commit(cfg, out);
    }
    if let Some(i) = remove {
        cfg.excludes.remove(i);
        commit(cfg, out);
    }

    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut state.new_exclude)
                .hint_text("node_modules")
                .desired_width(300.0),
        );
        if ui
            .add_enabled(
                !state.new_exclude.trim().is_empty(),
                egui::Button::new("Add exclude"),
            )
            .clicked()
        {
            cfg.excludes.push(state.new_exclude.trim().to_string());
            state.new_exclude.clear();
            commit(cfg, out);
        }
    });

    ui.add_space(16.0);
    if ui.button("Rebuild file index now").clicked() {
        out.catalog_stale = true;
        crate::logger::info("reindex requested from settings");
    }
}

fn bookmarks(
    ui: &mut egui::Ui,
    cfg: &mut Config,
    state: &mut SettingsState,
    out: &mut Outcome,
) {
    heading(
        ui,
        "Bookmarks",
        "Shown as suggestions when w  is typed with no query.",
    );

    let mut remove: Option<usize> = None;
    let mut changed = false;
    for (i, b) in cfg.bookmarks.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            changed |= ui
                .add(egui::TextEdit::singleline(&mut b.title).desired_width(180.0))
                .changed();
            changed |= ui
                .add(egui::TextEdit::singleline(&mut b.url).desired_width(340.0))
                .changed();
            if ui.button("×").clicked() {
                remove = Some(i);
            }
        });
    }
    if changed {
        commit(cfg, out);
    }
    if let Some(i) = remove {
        cfg.bookmarks.remove(i);
        commit(cfg, out);
    }

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut state.new_bookmark_title)
                .hint_text("Title")
                .desired_width(180.0),
        );
        ui.add(
            egui::TextEdit::singleline(&mut state.new_bookmark_url)
                .hint_text("https://…")
                .desired_width(340.0),
        );
        let valid = !state.new_bookmark_title.trim().is_empty()
            && !state.new_bookmark_url.trim().is_empty();
        if ui.add_enabled(valid, egui::Button::new("Add")).clicked() {
            cfg.bookmarks.push(Bookmark {
                title: state.new_bookmark_title.trim().to_string(),
                url: state.new_bookmark_url.trim().to_string(),
            });
            state.new_bookmark_title.clear();
            state.new_bookmark_url.clear();
            commit(cfg, out);
        }
    });
}

fn notes(ui: &mut egui::Ui, cfg: &mut Config, out: &mut Outcome) {
    heading(ui, "Notes", "Where .md files live.");

    ui.label("Notes folder");
    ui.horizontal(|ui| {
        let mut s = cfg.notes_dir.to_string_lossy().into_owned();
        if ui
            .add(egui::TextEdit::singleline(&mut s).desired_width(420.0))
            .changed()
        {
            cfg.notes_dir = std::path::PathBuf::from(s);
            commit(cfg, out);
        }
        if ui.button("Open").clicked() {
            let _ = std::fs::create_dir_all(&cfg.notes_dir);
            win::shell_open(&cfg.notes_dir.to_string_lossy());
        }
    });
}

fn blocklist(
    ui: &mut egui::Ui,
    cfg: &mut Config,
    state: &mut SettingsState,
    out: &mut Outcome,
) {
    heading(
        ui,
        "Blocklist",
        "Exact titles, exact paths, or substrings. Hidden items are excluded from every source.",
    );

    let mut remove: Option<usize> = None;
    let mut changed = false;
    for (i, e) in cfg.blocklist.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            changed |= ui
                .add(egui::TextEdit::singleline(e).desired_width(420.0))
                .changed();
            if ui.button("×").clicked() {
                remove = Some(i);
            }
        });
    }
    if changed {
        commit(cfg, out);
    }
    if let Some(i) = remove {
        cfg.blocklist.remove(i);
        commit(cfg, out);
    }

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut state.new_blocklist)
                .hint_text("Entry")
                .desired_width(420.0),
        );
        if ui
            .add_enabled(
                !state.new_blocklist.trim().is_empty(),
                egui::Button::new("Add"),
            )
            .clicked()
        {
            cfg.blocklist.push(state.new_blocklist.trim().to_string());
            state.new_blocklist.clear();
            commit(cfg, out);
        }
    });
}

fn about(ui: &mut egui::Ui, out: &mut Outcome) {
    heading(ui, "About Palette", "");
    ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
    ui.label(format!("Config: {}", Config::path().display()));
    ui.label(format!(
        "Log:    {}",
        crate::config::localappdata().join("log.txt").display()
    ));
    ui.label(format!(
        "WebView2 data: {}",
        crate::config::localappdata().join("WebView2").display()
    ));
    ui.add_space(12.0);

    ui.horizontal(|ui| {
        if ui.button("Open config folder").clicked() {
            win::shell_open(&crate::config::appdata().to_string_lossy());
        }
        if ui.button("Open log").clicked() {
            win::shell_open(&crate::config::localappdata().join("log.txt").to_string_lossy());
        }
    });
    ui.add_space(16.0);
    if ui.button("Quit Palette").clicked() {
        out.quit = true;
        out.close = true;
    }
}

use crate::config::Config;
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Apps,
    Commands,
    QuickFolders,
    Files,
    WindowsSettings,
}

impl Category {
    pub fn label(self) -> &'static str {
        match self {
            Category::Apps => "Apps",
            Category::Commands => "Commands",
            Category::QuickFolders => "Quick folders",
            Category::Files => "Files",
            Category::WindowsSettings => "Windows settings",
        }
    }
}

#[derive(Clone)]
pub enum Action {
    Open(String),
    MsSettings(String),
    Run { exe: String, args: String, cwd: String },
    Internal(&'static str),
}

#[derive(Clone)]
pub struct Item {
    pub title: String,
    pub subtitle: String,
    pub cat: Category,
    pub action: Action,
    pub haystack: String,
}

const SETTINGS: &[(&str, &str)] = &[
    ("Display settings", "ms-settings:display"),
    ("Sound settings", "ms-settings:sound"),
    ("Network settings", "ms-settings:network"),
    ("Bluetooth settings", "ms-settings:bluetooth"),
    ("Apps & features", "ms-settings:appsfeatures"),
    ("Windows Update", "ms-settings:windowsupdate"),
    ("Personalization", "ms-settings:personalization"),
    ("Privacy", "ms-settings:privacy"),
    ("Power & sleep", "ms-settings:powersleep"),
    ("Date & time", "ms-settings:dateandtime"),
    ("Storage", "ms-settings:storagesense"),
    ("Mouse", "ms-settings:mousetouchpad"),
];

pub fn build(cfg: &Config) -> Vec<Item> {
    let mut out = Vec::new();

    for root in start_menu_roots() {
        collect_lnks(&root, &mut out);
    }

    for p in &cfg.quick_folders {
        out.push(Item {
            title: p.file_name().map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| p.to_string_lossy().into_owned()),
            subtitle: p.to_string_lossy().into_owned(),
            cat: Category::QuickFolders,
            action: Action::Open(p.to_string_lossy().into_owned()),
            haystack: p.to_string_lossy().into_owned(),
        });
    }

    for c in &cfg.commands {
        out.push(Item {
            title: c.name.clone(),
            subtitle: c.exe.clone(),
            cat: Category::Commands,
            action: Action::Run { exe: c.exe.clone(), args: c.args.clone(), cwd: c.cwd.clone() },
            haystack: format!("{} {} {}", c.name, c.exe, c.keywords.join(" ")),
        });
    }

    for (name, internal) in [
        ("Settings", "settings"),
        ("Quit Palette", "quit"),
        ("Reload index", "reload"),
    ] {
        out.push(Item {
            title: name.into(),
            subtitle: "Palette".into(),
            cat: Category::Commands,
            action: Action::Internal(internal),
            haystack: name.into(),
        });
    }
    for (label, exe, args) in [
        ("Task Manager", "taskmgr.exe", ""),
        ("Control Panel", "control.exe", ""),
        ("Terminal", "wt.exe", ""),
        ("PowerShell", "powershell.exe", ""),
        ("Command Prompt", "cmd.exe", ""),
        ("Run dialog", "rundll32.exe", "shell32.dll,#61"),
    ] {
        out.push(Item {
            title: label.into(),
            subtitle: exe.into(),
            cat: Category::Commands,
            action: Action::Run { exe: exe.into(), args: args.into(), cwd: String::new() },
            haystack: label.into(),
        });
    }
    for (label, args) in [
        ("Lock", "/l"),
        ("Sleep", "/h"),
        ("Restart", "/r /t 0"),
        ("Shutdown", "/s /t 0"),
        ("Sign out", "/l"),
    ] {
        out.push(Item {
            title: label.into(),
            subtitle: "shutdown.exe".into(),
            cat: Category::Commands,
            action: Action::Run { exe: "shutdown.exe".into(), args: args.into(), cwd: String::new() },
            haystack: label.into(),
        });
    }

    for (name, uri) in SETTINGS {
        out.push(Item {
            title: (*name).into(),
            subtitle: (*uri).into(),
            cat: Category::WindowsSettings,
            action: Action::MsSettings((*uri).into()),
            haystack: format!("{} settings", name),
        });
    }

    out.retain(|i| {
        !(i.cat == Category::Commands
            && cfg.disabled_builtins.iter().any(|d| d == &i.title))
    });

    out.retain(|i| {
        !cfg.blocklist.iter().any(|b| {
            i.title.eq_ignore_ascii_case(b)
                || i.subtitle.eq_ignore_ascii_case(b)
                || i.title.to_lowercase().contains(&b.to_lowercase())
        })
    });

    out
}

fn start_menu_roots() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Some(p) = std::env::var_os("ProgramData") {
        v.push(PathBuf::from(p).join("Microsoft\\Windows\\Start Menu\\Programs"));
    }
    if let Some(p) = std::env::var_os("APPDATA") {
        v.push(PathBuf::from(p).join("Microsoft\\Windows\\Start Menu\\Programs"));
    }
    v
}

fn collect_lnks(root: &PathBuf, out: &mut Vec<Item>) {
    if !root.exists() {
        return;
    }
    for e in walkdir::WalkDir::new(root).max_depth(4).into_iter().flatten() {
        let p = e.path();
        if p.extension().map(|x| x.eq_ignore_ascii_case("lnk")).unwrap_or(false) {
            let name = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            if name.is_empty() || name.starts_with('.') {
                continue;
            }
            out.push(Item {
                title: name.clone(),
                subtitle: p.to_string_lossy().into_owned(),
                cat: Category::Apps,
                action: Action::Open(p.to_string_lossy().into_owned()),
                haystack: name,
            });
        }
    }
}

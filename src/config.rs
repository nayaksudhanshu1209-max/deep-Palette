use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Serialize, Deserialize)]
pub struct Bang {
    pub trigger: String,
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub keywords: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct UserCommand {
    pub name: String,
    pub exe: String,
    #[serde(default)]
    pub args: String,
    #[serde(default)]
    pub cwd: String,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub terminal: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Bookmark {
    pub title: String,
    pub url: String,
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Theme {
    Dark,
    Light,
    System,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Hotkey {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub vk: u32,
}

impl Default for Hotkey {
    fn default() -> Self {
        Self { ctrl: true, shift: false, alt: false, vk: 0x20 }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    pub hotkey: Hotkey,
    #[serde(default)]
    pub open_at_cursor: bool,
    #[serde(default)]
    pub compact: bool,
    #[serde(default = "yes")]
    pub show_suggestions_when_empty: bool,
    pub theme: Theme,
    #[serde(default = "one")]
    pub font_scale: f32,
    #[serde(default = "twenty")]
    pub max_results: usize,
    #[serde(default = "yes")]
    pub free_browser_memory: bool,
    #[serde(default)]
    pub low_memory_mode: bool,
    pub bangs: Vec<Bang>,
    #[serde(default)]
    pub commands: Vec<UserCommand>,
    #[serde(default)]
    pub quick_folders: Vec<PathBuf>,
    #[serde(default)]
    pub roots: Vec<PathBuf>,
    #[serde(default)]
    pub excludes: Vec<String>,
    #[serde(default)]
    pub bookmarks: Vec<Bookmark>,
    #[serde(default)]
    pub blocklist: Vec<String>,
    pub notes_dir: PathBuf,
    #[serde(default)]
    pub disabled_builtins: Vec<String>,
    #[serde(skip)]
    pub hotkey_fallback_note: Option<String>,
}

fn yes() -> bool { true }
fn one() -> f32 { 1.0 }
fn twenty() -> usize { 20 }

pub fn appdata() -> PathBuf {
    let p = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_default();
    let d = p.join("Palette");
    let _ = std::fs::create_dir_all(&d);
    d
}

pub fn localappdata() -> PathBuf {
    let p = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_default();
    let d = p.join("Palette");
    let _ = std::fs::create_dir_all(&d);
    d
}

impl Default for Config {
    fn default() -> Self {
        let mut roots = Vec::new();
        if let Some(p) = std::env::var_os("USERPROFILE") {
            let home = PathBuf::from(p);
            roots.push(home.join("Desktop"));
            roots.push(home.join("Documents"));
            roots.push(home.join("Downloads"));
        }
        Self {
            hotkey: Hotkey::default(),
            open_at_cursor: false,
            compact: false,
            show_suggestions_when_empty: true,
            theme: Theme::System,
            font_scale: 1.0,
            max_results: 20,
            free_browser_memory: true,
            low_memory_mode: false,
            bangs: vec![
                Bang { trigger: "g".into(), name: "Google".into(), url: "https://www.google.com/search?q={query}".into(), keywords: vec![] },
                Bang { trigger: "yt".into(), name: "YouTube".into(), url: "https://www.youtube.com/results?search_query={query}".into(), keywords: vec![] },
                Bang { trigger: "gh".into(), name: "GitHub".into(), url: "https://github.com/search?q={query}".into(), keywords: vec![] },
                Bang { trigger: "so".into(), name: "Stack Overflow".into(), url: "https://stackoverflow.com/search?q={query}".into(), keywords: vec![] },
                Bang { trigger: "wiki".into(), name: "Wikipedia".into(), url: "https://en.wikipedia.org/w/index.php?search={query}".into(), keywords: vec![] },
            ],
            commands: vec![],
            quick_folders: Vec::new(),
            roots,
            excludes: vec![
                ".git".into(), "node_modules".into(), "$Recycle.Bin".into(),
                "System Volume Information".into(),
                "AppData\\Local\\Temp".into(),
            ],
            bookmarks: vec![
                Bookmark { title: "GitHub".into(), url: "https://github.com".into() },
                Bookmark { title: "Wikipedia".into(), url: "https://en.wikipedia.org".into() },
            ],
            blocklist: Vec::new(),
            notes_dir: appdata().join("notes"),
            disabled_builtins: Vec::new(),
            hotkey_fallback_note: None,
        }
    }
}

impl Config {
    pub fn path() -> PathBuf { appdata().join("config.json") }

    pub fn load() -> Self {
        match std::fs::read_to_string(Self::path()) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
            Err(_) => {
                let c = Config::default();
                let _ = c.save();
                c
            }
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let tmp = Self::path().with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(self)?)?;
        std::fs::rename(tmp, Self::path())
    }
}

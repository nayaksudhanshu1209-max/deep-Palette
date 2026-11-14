use std::io::Write;
use std::sync::Mutex;

static LOCK: Mutex<()> = Mutex::new(());

pub fn init() {}

fn path() -> std::path::PathBuf {
    crate::config::localappdata().join("log.txt")
}

fn write(level: &str, msg: &str) {
    let _g = LOCK.lock();
    let p = path();
    if let Ok(m) = std::fs::metadata(&p) {
        if m.len() > 1_048_576 {
            let _ = std::fs::remove_file(&p);
        }
    }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&p) {
        let _ = writeln!(f, "[{level}] {msg}");
    }
}

pub fn info(m: &str) { write("info", m); }
pub fn warn(m: &str) { write("warn", m); }

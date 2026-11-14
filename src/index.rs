use std::path::PathBuf;
use std::sync::{Arc, RwLock};

pub struct FileIndex {
    files: RwLock<Vec<PathBuf>>,
}

impl FileIndex {
    pub fn new() -> Self {
        let mut files = Vec::new();
        if let Ok(s) = std::fs::read(crate::config::localappdata().join("index.bin")) {
            if let Ok(v) = serde_json::from_slice::<Vec<PathBuf>>(&s) {
                files = v;
            }
        }
        Self { files: RwLock::new(files) }
    }

    pub fn snapshot(&self) -> Vec<PathBuf> {
        self.files.read().map(|g| g.clone()).unwrap_or_default()
    }

    fn set(&self, v: Vec<PathBuf>) {
        if let Ok(mut g) = self.files.write() {
            *g = v;
        }
    }
}

pub fn spawn_indexer(
    roots: Vec<PathBuf>,
    excludes: Vec<String>,
    index: Arc<FileIndex>,
) {
    std::thread::Builder::new()
        .name("indexer".into())
        .spawn(move || {
            crate::win::below_normal_priority();
            let files = scan(&roots, &excludes);
            let p = crate::config::localappdata().join("index.bin");
            let tmp = p.with_extension("bin.tmp");
            if let Ok(bytes) = serde_json::to_vec(&files) {
                if std::fs::write(&tmp, bytes).is_ok() {
                    let _ = std::fs::rename(&tmp, &p);
                }
            }
            index.set(files);
        })
        .ok();
}

fn scan(roots: &[PathBuf], excludes: &[String]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for r in roots {
        if !r.exists() {
            continue;
        }
        let walker = walkdir::WalkDir::new(r)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                !excludes.iter().any(|x| name.eq_ignore_ascii_case(x))
                    && !e
                        .path()
                        .components()
                        .any(|c| c.as_os_str().to_string_lossy().starts_with('.'))
            });
        for e in walker.flatten() {
            if e.file_type().is_file() {
                out.push(e.path().to_path_buf());
                if out.len() >= 200_000 {
                    return out;
                }
            }
        }
    }
    out
}

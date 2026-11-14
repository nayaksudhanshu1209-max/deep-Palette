use crate::config::Config;
use std::path::PathBuf;

pub struct NoteState {
    pub path: PathBuf,
    pub text: String,
    pub dirty: bool,
    pub last_save: std::time::Instant,
}

impl NoteState {
    pub fn open(cfg: &Config, name: &str) -> Self {
        let _ = std::fs::create_dir_all(&cfg.notes_dir);
        let file = if name.trim().is_empty() {
            most_recent(&cfg.notes_dir).unwrap_or_else(|| cfg.notes_dir.join("untitled.md"))
        } else {
            let mut n = name.trim().to_string();
            if !n.ends_with(".md") {
                n.push_str(".md");
            }
            cfg.notes_dir.join(n)
        };
        let text = std::fs::read_to_string(&file).unwrap_or_default();
        Self { path: file, text, dirty: false, last_save: std::time::Instant::now() }
    }

    pub fn word_count(&self) -> usize {
        self.text.split_whitespace().count()
    }

    pub fn save(&mut self) -> std::io::Result<()> {
        if !self.dirty {
            return Ok(());
        }
        if let Some(p) = self.path.parent() {
            let _ = std::fs::create_dir_all(p);
        }
        let tmp = self.path.with_extension("md.tmp");
        std::fs::write(&tmp, &self.text)?;
        std::fs::rename(tmp, &self.path)?;
        self.dirty = false;
        self.last_save = std::time::Instant::now();
        Ok(())
    }
}

fn most_recent(dir: &PathBuf) -> Option<PathBuf> {
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for e in std::fs::read_dir(dir).ok()?.flatten() {
        let p = e.path();
        if p.extension().map(|x| x == "md").unwrap_or(false) {
            if let Ok(m) = e.metadata().and_then(|m| m.modified()) {
                if best.as_ref().map_or(true, |(b, _)| m > *b) {
                    best = Some((m, p));
                }
            }
        }
    }
    best.map(|(_, p)| p)
}

pub fn editor(ui: &mut egui::Ui, state: &mut NoteState) -> bool {
    let mut close = false;

    if state.dirty && state.last_save.elapsed().as_secs() >= 5 {
        let _ = state.save();
    }

    if ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::W)) {
        close = true;
    }

    let mut layouter = |ui: &egui::Ui, s: &dyn egui::TextBuffer, _wrap: f32| {
        let text = s.as_str().to_string();
        let job = markdown_job(ui, &text);
        ui.fonts(|f| f.layout_job(job))
    };

    let out = egui::TextEdit::multiline(&mut state.text)
        .id(egui::Id::new("note_editor"))
        .font(egui::FontId::proportional(14.0))
        .desired_width(f32::INFINITY)
        .desired_rows(30)
        .layouter(&mut layouter)
        .show(ui);

    if out.response.changed() {
        state.dirty = true;
    }
    close
}

fn markdown_job(ui: &egui::Ui, text: &str) -> egui::text::LayoutJob {
    use egui::text::{LayoutJob, TextFormat};
    let mut job = LayoutJob::default();
    let base = TextFormat {
        font_id: egui::FontId::proportional(14.0),
        color: ui.visuals().text_color(),
        ..Default::default()
    };
    let dim = TextFormat {
        font_id: egui::FontId::monospace(13.0),
        color: ui.visuals().weak_text_color(),
        ..Default::default()
    };
    let bold = TextFormat {
        font_id: egui::FontId::proportional(15.0),
        color: ui.visuals().strong_text_color(),
        ..Default::default()
    };
    let head = TextFormat {
        font_id: egui::FontId::proportional(20.0),
        color: ui.visuals().strong_text_color(),
        ..Default::default()
    };

    for line in text.split_inclusive('\n') {
        if line.starts_with("# ") {
            job.append("# ", 0.0, dim.clone());
            job.append(&line[2..], 0.0, head.clone());
            continue;
        }
        if line.starts_with("- ") {
            job.append("- ", 0.0, dim.clone());
            job.append(&line[2..], 0.0, base.clone());
            continue;
        }
        if line.starts_with("> ") {
            job.append("> ", 0.0, dim.clone());
            job.append(&line[2..], 0.0, base.clone());
            continue;
        }
        let mut rest = line;
        while let Some(i) = rest.find("**") {
            job.append(&rest[..i], 0.0, base.clone());
            if let Some(j) = rest[i + 2..].find("**") {
                job.append("**", 0.0, dim.clone());
                job.append(&rest[i + 2..i + 2 + j], 0.0, bold.clone());
                job.append("**", 0.0, dim.clone());
                rest = &rest[i + 2 + j + 2..];
            } else {
                job.append(&rest[i..], 0.0, base.clone());
                rest = "";
            }
        }
        job.append(rest, 0.0, base.clone());
    }
    job
}

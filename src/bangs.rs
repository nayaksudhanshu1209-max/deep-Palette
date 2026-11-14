use crate::config::Bang;

#[derive(Clone, Copy, PartialEq)]
pub enum ChipKind {
    Bang(usize),
    Web,
    Note,
    Settings,
}

#[derive(Clone)]
pub struct ChipInfo {
    pub kind: ChipKind,
    pub label: String,
}

const RESERVED: &[(&str, &str, ChipKind)] = &[
    ("wb", "Web", ChipKind::Web),
    ("w", "Web", ChipKind::Web),
    ("note", "Note", ChipKind::Note),
    ("settings", "Settings", ChipKind::Settings),
];

pub fn parse(raw: &str, bangs: &[Bang]) -> Option<(ChipInfo, usize)> {
    let (input, bang_prefix_len) = match raw.strip_prefix('!') {
        Some(rest) => (rest, 1),
        None => (raw, 0),
    };

    let mut best: Option<(ChipInfo, usize)> = None;

    let mut consider = |trigger: &str, label: String, kind: ChipKind| {
        let tlen = trigger.len();
        if input.len() <= tlen {
            return;
        }
        if !input.as_bytes()[tlen].eq(&b' ') {
            return;
        }
        if !input[..tlen].eq_ignore_ascii_case(trigger) {
            return;
        }
        let off = bang_prefix_len + tlen + 1;
        let cand = (ChipInfo { kind, label }, off);
        if best.as_ref().map_or(true, |(_, o)| off > *o) {
            best = Some(cand);
        }
    };

    for (t, label, kind) in RESERVED {
        consider(t, label.to_string(), *kind);
    }
    for (i, b) in bangs.iter().enumerate() {
        consider(&b.trigger, b.name.clone(), ChipKind::Bang(i));
    }
    best
}

pub fn is_reserved(t: &str) -> bool {
    RESERVED.iter().any(|(r, _, _)| r.eq_ignore_ascii_case(t))
}

use crate::sources::Item;
use nucleo_matcher::{
    pattern::{CaseMatching, Normalization, Pattern},
    Config, Matcher, Utf32Str,
};

pub fn rank(query: &str, items: &[Item], max: usize) -> Vec<(u32, usize)> {
    let mut matcher = Matcher::new(Config::DEFAULT);
    let pat = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);
    let mut buf = Vec::new();
    let mut scored: Vec<(u32, usize)> = Vec::new();
    for (i, it) in items.iter().enumerate() {
        let hay = Utf32Str::new(&it.haystack, &mut buf);
        if let Some(s) = pat.score(hay, &mut matcher) {
            scored.push((s, i));
        }
    }
    scored.sort_by(|a, b| b.0.cmp(&a.0));
    scored.truncate(max);
    scored
}

//! Diacritic-insensitive fuzzy matching, ported 1:1 from the Python plugin.

use unicode_general_category::{get_general_category, GeneralCategory};
use unicode_normalization::UnicodeNormalization;

/// Remove diacritics (e.g. á→a, ž→z) and lowercase.
fn strip_diacritics(text: &str) -> String {
    text.nfkd()
        .filter(|c| get_general_category(*c) != GeneralCategory::NonspacingMark)
        .collect::<String>()
        .to_lowercase()
}

/// Score how well `query` fuzzy-matches `text`. `None` if not all query chars
/// are found in order.
pub fn fuzzy_score(query: &str, text: &str) -> Option<i32> {
    let query = strip_diacritics(query);
    let text_norm = strip_diacritics(text);
    if query.is_empty() {
        return Some(0);
    }

    let q: Vec<char> = query.chars().collect();
    let t: Vec<char> = text_norm.chars().collect();
    let mut qi = 0usize;
    let mut score = 0i32;
    let mut prev_match: i64 = -2;

    for (ti, &tc) in t.iter().enumerate() {
        if qi < q.len() && tc == q[qi] {
            if ti as i64 == prev_match + 1 {
                score += 3; // consecutive chars
            } else if ti == 0 || matches!(t[ti - 1], ' ' | '_' | '-' | '.' | '/') {
                score += 2; // word boundary
            } else {
                score += 1;
            }
            prev_match = ti as i64;
            qi += 1;
        }
    }

    if qi < q.len() {
        return None; // not all query chars found
    }

    // Prefer tighter matches (shorter target text).
    score += std::cmp::max(0, 10 - (t.len() as i32 - q.len() as i32));
    Some(score)
}

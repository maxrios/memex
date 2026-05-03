use anyhow::Result;

use crate::store::GraphStore;

pub fn run(query: &str) -> Result<()> {
    let store = GraphStore::open_from_cwd()?;
    let nodes = store.load_all_nodes()?;
    let state = store.load_state()?;

    let query_lower = query.to_lowercase();
    let mut found = false;

    for node in &nodes {
        let mut matched_fields: Vec<String> = Vec::new();

        // Search goal
        if node.summary.goal.to_lowercase().contains(&query_lower) {
            matched_fields.push(format!(
                "goal: \"{}\"",
                highlight(&node.summary.goal, query)
            ));
        }

        // Search decisions
        for (i, d) in node.summary.decisions.iter().enumerate() {
            if d.to_lowercase().contains(&query_lower) {
                matched_fields.push(format!("decision[{}]: \"{}\"", i, highlight(d, query)));
            }
        }

        // Search rejected approaches
        for (i, r) in node.summary.rejected_approaches.iter().enumerate() {
            if r.description.to_lowercase().contains(&query_lower) {
                matched_fields.push(format!(
                    "rejected[{}].description: \"{}\"",
                    i,
                    highlight(&r.description, query)
                ));
            }
            if r.reason.to_lowercase().contains(&query_lower) {
                matched_fields.push(format!(
                    "rejected[{}].reason: \"{}\"",
                    i,
                    highlight(&r.reason, query)
                ));
            }
        }

        // Search open threads
        for (i, t) in node.summary.open_threads.iter().enumerate() {
            if t.to_lowercase().contains(&query_lower) {
                matched_fields.push(format!("open_thread[{}]: \"{}\"", i, highlight(t, query)));
            }
        }

        // Search key artifacts
        for (i, a) in node.summary.key_artifacts.iter().enumerate() {
            if a.to_lowercase().contains(&query_lower) {
                matched_fields.push(format!("artifact[{}]: \"{}\"", i, highlight(a, query)));
            }
        }

        // Search tags
        for tag in &node.tags {
            if tag.to_lowercase().contains(&query_lower) {
                matched_fields.push(format!("tag: \"{}\"", highlight(tag, query)));
            }
        }

        if !matched_fields.is_empty() {
            found = true;
            let active_marker = if state.active_id == Some(node.id) {
                " [*]"
            } else {
                ""
            };
            println!(
                "Node: {} ({}){}",
                node.short_id(),
                node.status,
                active_marker
            );
            for field in &matched_fields {
                println!("  {}", field);
            }
            println!();
        }
    }

    if !found {
        println!("No nodes found matching '{}'.", query);
    }

    Ok(())
}

/// Simple case-insensitive highlight: wraps matches in >> << markers.
pub(crate) fn highlight(text: &str, query: &str) -> String {
    let lower = text.to_lowercase();
    let query_lower = query.to_lowercase();
    let query_lower_chars: Vec<char> = query_lower.chars().collect();
    let ql = query_lower_chars.len();

    if ql == 0 {
        return text.to_string();
    }

    // Work in char space to avoid byte-boundary panics from Unicode case folding:
    // to_lowercase() can change byte length (e.g. ẞ [3 bytes] → ß [2 bytes]),
    // so byte offsets from `lower` are not valid slice indices into `text`.
    let text_chars: Vec<char> = text.chars().collect();
    let lower_chars: Vec<char> = lower.chars().collect();
    let n = text_chars.len();

    // Rare case: folding expanded char count (e.g. İ → i + combining dot).
    // Return unmodified — no highlight, but no panic either.
    if n != lower_chars.len() {
        return text.to_string();
    }

    let mut result = String::with_capacity(text.len());
    let mut i = 0;
    while i < n {
        if i + ql <= n && lower_chars[i..i + ql] == query_lower_chars[..] {
            result.push_str(">>");
            result.extend(text_chars[i..i + ql].iter());
            result.push_str("<<");
            i += ql;
        } else {
            result.push(text_chars[i]);
            i += 1;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlight_exact_match() {
        assert_eq!(highlight("hello world", "world"), "hello >>world<<");
    }

    #[test]
    fn highlight_case_insensitive() {
        assert_eq!(highlight("Hello World", "hello"), ">>Hello<< World");
    }

    #[test]
    fn highlight_multiple_occurrences() {
        assert_eq!(highlight("foo bar foo", "foo"), ">>foo<< bar >>foo<<");
    }

    #[test]
    fn highlight_no_match() {
        assert_eq!(highlight("hello world", "xyz"), "hello world");
    }

    #[test]
    fn highlight_empty_text() {
        assert_eq!(highlight("", "foo"), "");
    }

    #[test]
    fn highlight_match_at_start() {
        assert_eq!(highlight("start of string", "start"), ">>start<< of string");
    }

    #[test]
    fn highlight_match_at_end() {
        assert_eq!(highlight("at the end", "end"), "at the >>end<<");
    }

    #[test]
    fn highlight_entire_string() {
        assert_eq!(highlight("exact", "exact"), ">>exact<<");
    }

    #[test]
    fn highlight_multibyte_utf8_smoke() {
        // "café" has a 2-byte é; the match is after it — verify no byte-boundary panic
        assert_eq!(highlight("café latte", "latte"), "café >>latte<<");
    }

    #[test]
    fn highlight_unicode_folding_byte_shift_no_panic() {
        // ẞ (U+1E9E, 3 bytes) folds to ß (U+00DF, 2 bytes) — a match queried as
        // "ß" must not panic when indexing back into the original text.
        assert_eq!(highlight("STRAẞE", "ß"), "STRA>>ẞ<<E");
    }

    #[test]
    fn highlight_unicode_match_after_folded_char() {
        // ẞ (3 bytes) → ß (2 bytes) shifts all byte offsets after it. A match
        // that starts after ẞ used to panic with the byte-index approach.
        assert_eq!(highlight("ẞtraße", "tra"), "ẞ>>tra<<ße");
    }
}

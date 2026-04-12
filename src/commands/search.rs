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
    let mut result = String::new();
    let mut pos = 0;

    while let Some(idx) = lower[pos..].find(&query_lower) {
        let abs_idx = pos + idx;
        result.push_str(&text[pos..abs_idx]);
        result.push_str(">>");
        result.push_str(&text[abs_idx..abs_idx + query.len()]);
        result.push_str("<<");
        pos = abs_idx + query.len();
    }
    result.push_str(&text[pos..]);
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
}

use std::collections::HashSet;

use anyhow::Result;

use crate::models::{ConversationNode, NodeStatus};
use crate::store::GraphStore;

pub const DEFAULT_MARKER: &str = "<!-- memex-pr-context -->";
pub const DEFAULT_LIMIT: usize = 5;

/// Find nodes whose `key_artifacts` exact-match any of the given file paths.
/// Returns nodes sorted by `created_at` descending (most recent first).
pub(crate) fn matching_nodes<'a>(
    nodes: &'a [ConversationNode],
    files: &[String],
) -> Vec<&'a ConversationNode> {
    let file_set: HashSet<&str> = files.iter().map(|f| f.as_str()).collect();
    let mut matches: Vec<&ConversationNode> = nodes
        .iter()
        .filter(|n| {
            n.summary
                .key_artifacts
                .iter()
                .any(|a| file_set.contains(a.as_str()))
        })
        .collect();
    matches.sort_by_key(|n| std::cmp::Reverse(n.created_at));
    matches
}

/// Format the matched nodes as a markdown PR comment body.
/// `limit` caps the number of nodes rendered (0 = unlimited). When the cap
/// triggers, a trailing line announces how many older matches were omitted.
pub(crate) fn format_comment(matches: &[&ConversationNode], marker: &str, limit: usize) -> String {
    let mut out = String::new();
    out.push_str(marker);
    out.push('\n');
    out.push_str("## memex context for this PR\n\n");

    if matches.is_empty() {
        out.push_str("_No prior memex nodes touched the changed files._\n");
        return out;
    }

    out.push_str(
        "Prior memex nodes that touched files in this PR. Use this to see what was tried, \
         rejected, or left open in earlier work on these files.\n\n",
    );

    let (rendered, omitted) = if limit == 0 || matches.len() <= limit {
        (matches, 0usize)
    } else {
        (&matches[..limit], matches.len() - limit)
    };

    for node in rendered {
        let status_label = match node.status {
            NodeStatus::Active => "Active",
            NodeStatus::Resolved => "Resolved",
            NodeStatus::Abandoned => "Abandoned",
        };
        out.push_str(&format!(
            "### `{}` — {} ({})\n\n",
            node.short_id(),
            node.summary.goal,
            status_label
        ));

        if !node.summary.key_artifacts.is_empty() {
            out.push_str("**Artifacts:** ");
            out.push_str(&node.summary.key_artifacts.join(", "));
            out.push_str("\n\n");
        }

        if !node.summary.decisions.is_empty() {
            out.push_str("**Decisions:**\n");
            for d in &node.summary.decisions {
                out.push_str(&format!("- {}\n", d));
            }
            out.push('\n');
        }

        if !node.summary.rejected_approaches.is_empty() {
            out.push_str("**Rejected approaches:**\n");
            for r in &node.summary.rejected_approaches {
                out.push_str(&format!("- {} — _{}_\n", r.description, r.reason));
            }
            out.push('\n');
        }

        if !node.summary.open_threads.is_empty() {
            out.push_str("**Open threads:**\n");
            for t in &node.summary.open_threads {
                out.push_str(&format!("- {}\n", t));
            }
            out.push('\n');
        }
    }

    if omitted > 0 {
        out.push_str(&format!(
            "_{} older matching node{} omitted; raise `--limit` to see more._\n",
            omitted,
            if omitted == 1 { "" } else { "s" }
        ));
    }

    out
}

pub fn run(files: &[String], marker: &str, limit: usize) -> Result<()> {
    let store = GraphStore::open_from_cwd()?;
    let nodes = store.load_all_nodes()?;
    let matches = matching_nodes(&nodes, files);
    let body = format_comment(&matches, marker, limit);
    print!("{}", body);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{NodeStatus, NodeSummary, RejectedApproach};
    use chrono::{Duration, TimeZone, Utc};
    use uuid::Uuid;

    fn make_node(goal: &str, artifacts: Vec<&str>, created_offset_secs: i64) -> ConversationNode {
        let base = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        ConversationNode {
            id: Uuid::new_v4(),
            parent_ids: vec![],
            git_ref: None,
            created_at: base + Duration::seconds(created_offset_secs),
            updated_at: base + Duration::seconds(created_offset_secs),
            summary: NodeSummary {
                goal: goal.to_string(),
                decisions: vec![],
                rejected_approaches: vec![],
                open_threads: vec![],
                key_artifacts: artifacts.into_iter().map(String::from).collect(),
            },
            raw_transcript_ref: None,
            tags: vec![],
            status: NodeStatus::Resolved,
        }
    }

    #[test]
    fn matches_exact_artifact_path() {
        let nodes = vec![
            make_node("touched main", vec!["src/main.rs"], 0),
            make_node("touched store", vec!["src/store.rs"], 10),
        ];
        let files = vec!["src/main.rs".to_string()];
        let m = matching_nodes(&nodes, &files);
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].summary.goal, "touched main");
    }

    #[test]
    fn no_match_returns_empty() {
        let nodes = vec![make_node("touched main", vec!["src/main.rs"], 0)];
        let files = vec!["src/other.rs".to_string()];
        assert!(matching_nodes(&nodes, &files).is_empty());
    }

    #[test]
    fn match_is_case_sensitive() {
        let nodes = vec![make_node("touched main", vec!["src/Main.rs"], 0)];
        let files = vec!["src/main.rs".to_string()];
        assert!(matching_nodes(&nodes, &files).is_empty());
    }

    #[test]
    fn match_does_not_use_directory_prefix() {
        // "src" should not match "src/main.rs" — exact only.
        let nodes = vec![make_node("touched main", vec!["src/main.rs"], 0)];
        let files = vec!["src".to_string()];
        assert!(matching_nodes(&nodes, &files).is_empty());
    }

    #[test]
    fn matches_sorted_by_created_at_desc() {
        let nodes = vec![
            make_node("oldest", vec!["src/main.rs"], 0),
            make_node("newest", vec!["src/main.rs"], 100),
            make_node("middle", vec!["src/main.rs"], 50),
        ];
        let files = vec!["src/main.rs".to_string()];
        let m = matching_nodes(&nodes, &files);
        assert_eq!(m.len(), 3);
        assert_eq!(m[0].summary.goal, "newest");
        assert_eq!(m[1].summary.goal, "middle");
        assert_eq!(m[2].summary.goal, "oldest");
    }

    #[test]
    fn dedup_node_with_multiple_matching_artifacts() {
        // A node with two artifacts both in the changed-file set must appear once.
        let nodes = vec![make_node(
            "touched both",
            vec!["src/main.rs", "src/store.rs"],
            0,
        )];
        let files = vec!["src/main.rs".to_string(), "src/store.rs".to_string()];
        let m = matching_nodes(&nodes, &files);
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn empty_body_for_no_matches() {
        let body = format_comment(&[], DEFAULT_MARKER, DEFAULT_LIMIT);
        assert!(body.starts_with(DEFAULT_MARKER));
        assert!(body.contains("No prior memex nodes touched"));
    }

    #[test]
    fn body_includes_marker_first_line() {
        let body = format_comment(&[], DEFAULT_MARKER, DEFAULT_LIMIT);
        let first_line = body.lines().next().unwrap();
        assert_eq!(first_line, DEFAULT_MARKER);
    }

    #[test]
    fn body_renders_node_sections() {
        let mut node = make_node("ship the feature", vec!["src/main.rs"], 0);
        node.summary.decisions.push("chose A".to_string());
        node.summary.rejected_approaches.push(RejectedApproach {
            description: "B".to_string(),
            reason: "too slow".to_string(),
        });
        node.summary
            .open_threads
            .push("revisit logging".to_string());

        let body = format_comment(&[&node], DEFAULT_MARKER, DEFAULT_LIMIT);
        assert!(body.contains("ship the feature"));
        assert!(body.contains("**Artifacts:** src/main.rs"));
        assert!(body.contains("**Decisions:**"));
        assert!(body.contains("- chose A"));
        assert!(body.contains("**Rejected approaches:**"));
        assert!(body.contains("B — _too slow_"));
        assert!(body.contains("**Open threads:**"));
        assert!(body.contains("- revisit logging"));
    }

    #[test]
    fn body_omits_empty_sections() {
        let node = make_node("bare node", vec!["src/main.rs"], 0);
        let body = format_comment(&[&node], DEFAULT_MARKER, DEFAULT_LIMIT);
        assert!(body.contains("bare node"));
        assert!(!body.contains("**Decisions:**"));
        assert!(!body.contains("**Rejected approaches:**"));
        assert!(!body.contains("**Open threads:**"));
    }

    #[test]
    fn custom_marker_used() {
        let body = format_comment(&[], "<!-- custom -->", DEFAULT_LIMIT);
        assert!(body.starts_with("<!-- custom -->"));
    }

    #[test]
    fn limit_caps_rendered_nodes_and_announces_omitted_count() {
        let nodes: Vec<ConversationNode> = (0..7)
            .map(|i| make_node(&format!("node {}", i), vec!["src/main.rs"], i * 10))
            .collect();
        let refs: Vec<&ConversationNode> = nodes.iter().collect();

        let body = format_comment(&refs, DEFAULT_MARKER, 3);
        // First three by position should appear; tail should be omitted.
        assert!(body.contains("node 0"));
        assert!(body.contains("node 1"));
        assert!(body.contains("node 2"));
        assert!(!body.contains("node 3"));
        assert!(body.contains("4 older matching nodes omitted"));
    }

    #[test]
    fn limit_zero_renders_all_nodes() {
        let nodes: Vec<ConversationNode> = (0..3)
            .map(|i| make_node(&format!("node {}", i), vec!["src/main.rs"], i * 10))
            .collect();
        let refs: Vec<&ConversationNode> = nodes.iter().collect();

        let body = format_comment(&refs, DEFAULT_MARKER, 0);
        assert!(body.contains("node 0"));
        assert!(body.contains("node 1"));
        assert!(body.contains("node 2"));
        assert!(!body.contains("omitted"));
    }

    #[test]
    fn limit_at_or_above_match_count_omits_no_footer() {
        let nodes: Vec<ConversationNode> = (0..3)
            .map(|i| make_node(&format!("node {}", i), vec!["src/main.rs"], i * 10))
            .collect();
        let refs: Vec<&ConversationNode> = nodes.iter().collect();

        let body = format_comment(&refs, DEFAULT_MARKER, 3);
        assert!(!body.contains("omitted"));
        let body_above = format_comment(&refs, DEFAULT_MARKER, 99);
        assert!(!body_above.contains("omitted"));
    }

    #[test]
    fn limit_one_uses_singular_in_footer() {
        let nodes: Vec<ConversationNode> = (0..2)
            .map(|i| make_node(&format!("node {}", i), vec!["src/main.rs"], i * 10))
            .collect();
        let refs: Vec<&ConversationNode> = nodes.iter().collect();

        let body = format_comment(&refs, DEFAULT_MARKER, 1);
        assert!(body.contains("1 older matching node omitted"));
    }
}

use std::collections::HashMap;

use anyhow::Result;
use uuid::Uuid;

use crate::models::ConversationNode;
use crate::store::GraphStore;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutputFormat {
    Markdown,
    Xml,
    Plain,
}

impl OutputFormat {
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "markdown" | "md" => Ok(OutputFormat::Markdown),
            "xml" => Ok(OutputFormat::Xml),
            "plain" | "text" => Ok(OutputFormat::Plain),
            _ => anyhow::bail!("Unknown format '{}'. Use: markdown, xml, plain", s),
        }
    }
}

pub fn run(id: Option<&str>, format: OutputFormat, depth: usize) -> Result<()> {
    let store = GraphStore::open_from_cwd()?;
    let node_id = store.resolve_node_id(id)?;
    let nodes = store.load_all_nodes()?;
    let node_map: HashMap<Uuid, &ConversationNode> = nodes.iter().map(|n| (n.id, n)).collect();

    let root_id = find_root(&nodes)?;

    let path = match find_path(root_id, node_id, &node_map) {
        Some(p) => p,
        None => {
            eprintln!(
                "warning: node {} is not reachable from the graph root — ancestor context will be incomplete",
                &node_id.to_string()[..8]
            );
            vec![node_id]
        }
    };

    let path = trim_path(path, depth);

    let output = match format {
        OutputFormat::Markdown => generate_markdown(&path, &node_map, node_id)?,
        OutputFormat::Xml => generate_xml(&path, &node_map)?,
        OutputFormat::Plain => generate_plain(&path, &node_map)?,
    };

    println!("{}", output);
    Ok(())
}

/// The root is the unique node with empty `parent_ids`. Anything else
/// indicates an empty store or a corrupted graph (e.g. an orphan node
/// created by hand-editing JSON).
pub(crate) fn find_root(nodes: &[ConversationNode]) -> Result<Uuid> {
    let roots: Vec<Uuid> = nodes
        .iter()
        .filter(|n| n.parent_ids.is_empty())
        .map(|n| n.id)
        .collect();
    match roots.as_slice() {
        [single] => Ok(*single),
        [] => anyhow::bail!("No root node found. Run `memex init`."),
        multiple => anyhow::bail!(
            "Graph has {} root nodes (expected 1). A node with empty parent_ids was likely created by hand-editing.",
            multiple.len()
        ),
    }
}

/// Trim the ancestor section (path[1..len-1]) to at most `depth` nodes,
/// always keeping the root (path[0]) and target (path[last]).
pub(crate) fn trim_path(path: Vec<Uuid>, depth: usize) -> Vec<Uuid> {
    if path.len() <= 2 {
        return path;
    }
    let ancestors = &path[1..path.len() - 1];
    if ancestors.len() <= depth {
        return path;
    }
    let skip = ancestors.len() - depth;
    let mut trimmed = vec![path[0]];
    trimmed.extend_from_slice(&ancestors[skip..]);
    trimmed.push(*path.last().unwrap());
    trimmed
}

/// Find the path (list of node IDs) from `start` to `target` using BFS.
pub(crate) fn find_path(
    start: Uuid,
    target: Uuid,
    node_map: &HashMap<Uuid, &ConversationNode>,
) -> Option<Vec<Uuid>> {
    if start == target {
        return Some(vec![start]);
    }

    let mut queue: std::collections::VecDeque<Vec<Uuid>> = std::collections::VecDeque::new();
    queue.push_back(vec![start]);

    let mut visited = std::collections::HashSet::new();
    visited.insert(start);

    while let Some(path) = queue.pop_front() {
        let current = *path.last().unwrap();
        if let Some(_node) = node_map.get(&current) {
            // Get children by looking at all nodes whose parent_ids contain current
            for (id, n) in node_map.iter() {
                if n.parent_ids.contains(&current) && !visited.contains(id) {
                    let mut new_path = path.clone();
                    new_path.push(*id);
                    if *id == target {
                        return Some(new_path);
                    }
                    visited.insert(*id);
                    queue.push_back(new_path);
                }
            }
        }
    }

    None
}

fn generate_markdown(
    path: &[Uuid],
    node_map: &HashMap<Uuid, &ConversationNode>,
    target_id: Uuid,
) -> Result<String> {
    let mut out = String::new();

    if path.is_empty() {
        return Ok(out);
    }

    // Root node
    let root_id = path[0];
    if let Some(root) = node_map.get(&root_id) {
        out.push_str("## Project Context\n\n");
        out.push_str(&format!("**Goal:** {}\n\n", root.summary.goal));
        if !root.summary.decisions.is_empty() {
            out.push_str("**Key Decisions:**\n");
            for d in &root.summary.decisions {
                out.push_str(&format!("- {}\n", d));
            }
            out.push('\n');
        }
        if !root.summary.key_artifacts.is_empty() {
            out.push_str("**Key Artifacts:** ");
            out.push_str(&root.summary.key_artifacts.join(", "));
            out.push_str("\n\n");
        }
    }

    // Ancestors (everything between root and immediate parent of target)
    let ancestor_range = if path.len() > 2 {
        &path[1..path.len() - 1]
    } else {
        &[]
    };
    if !ancestor_range.is_empty() {
        out.push_str("## Ancestor Context\n\n");
        for &anc_id in ancestor_range {
            if let Some(node) = node_map.get(&anc_id) {
                let short = &node.id.to_string()[..8];
                out.push_str(&format!("- [node {}] Goal: {}", short, node.summary.goal));
                if !node.summary.decisions.is_empty() {
                    out.push_str(&format!(
                        " | Decisions: {}",
                        node.summary.decisions.join("; ")
                    ));
                }
                out.push('\n');
            }
        }
        out.push('\n');
    }

    // Immediate parent (or target itself if no ancestors)
    let immediate = if path.len() > 1 {
        path[path.len() - 1]
    } else {
        target_id
    };

    // If path.len() > 1, the last element IS the target; show it as "Current Node"
    // If path.len() == 1, it's both root and target
    let section_title = if path.len() == 1 || (path.len() > 1 && path[path.len() - 1] == target_id)
    {
        "## Current Node Context"
    } else {
        "## Immediate Parent Context"
    };

    if let Some(node) = node_map.get(&immediate) {
        out.push_str(&format!("{}\n\n", section_title));
        out.push_str(&format!("**Goal:** {}\n\n", node.summary.goal));

        if !node.summary.decisions.is_empty() {
            out.push_str("**Decisions:**\n");
            for d in &node.summary.decisions {
                out.push_str(&format!("- {}\n", d));
            }
            out.push('\n');
        }

        if !node.summary.rejected_approaches.is_empty() {
            out.push_str("**Rejected Approaches:**\n");
            for r in &node.summary.rejected_approaches {
                out.push_str(&format!("- {} — _{}_\n", r.description, r.reason));
            }
            out.push('\n');
        }

        if !node.summary.open_threads.is_empty() {
            out.push_str("**Open Threads:**\n");
            for t in &node.summary.open_threads {
                out.push_str(&format!("- {}\n", t));
            }
            out.push('\n');
        }

        if !node.summary.key_artifacts.is_empty() {
            out.push_str(&format!(
                "**Key Artifacts:** {}\n",
                node.summary.key_artifacts.join(", ")
            ));
        }
    }

    Ok(out)
}

fn generate_xml(path: &[Uuid], node_map: &HashMap<Uuid, &ConversationNode>) -> Result<String> {
    let mut out = String::new();
    out.push_str("<memex_context>\n");

    if path.is_empty() {
        out.push_str("</memex_context>\n");
        return Ok(out);
    }

    // Root
    let root_id = path[0];
    if let Some(root) = node_map.get(&root_id) {
        out.push_str("  <project_context>\n");
        out.push_str(&format!(
            "    <goal>{}</goal>\n",
            xml_escape(&root.summary.goal)
        ));
        if !root.summary.decisions.is_empty() {
            out.push_str("    <decisions>\n");
            for d in &root.summary.decisions {
                out.push_str(&format!("      <decision>{}</decision>\n", xml_escape(d)));
            }
            out.push_str("    </decisions>\n");
        }
        out.push_str("  </project_context>\n");
    }

    // Ancestors
    let ancestor_range = if path.len() > 2 {
        &path[1..path.len() - 1]
    } else {
        &[]
    };
    if !ancestor_range.is_empty() {
        out.push_str("  <ancestor_context>\n");
        for &anc_id in ancestor_range {
            if let Some(node) = node_map.get(&anc_id) {
                let short = &node.id.to_string()[..8];
                out.push_str(&format!("    <ancestor id=\"{}\">\n", short));
                out.push_str(&format!(
                    "      <goal>{}</goal>\n",
                    xml_escape(&node.summary.goal)
                ));
                if !node.summary.decisions.is_empty() {
                    out.push_str("      <decisions>\n");
                    for d in &node.summary.decisions {
                        out.push_str(&format!("        <decision>{}</decision>\n", xml_escape(d)));
                    }
                    out.push_str("      </decisions>\n");
                }
                out.push_str("    </ancestor>\n");
            }
        }
        out.push_str("  </ancestor_context>\n");
    }

    // Immediate/current
    let immediate = path[path.len() - 1];
    if let Some(node) = node_map.get(&immediate) {
        out.push_str("  <current_node_context>\n");
        out.push_str(&format!(
            "    <goal>{}</goal>\n",
            xml_escape(&node.summary.goal)
        ));

        if !node.summary.decisions.is_empty() {
            out.push_str("    <decisions>\n");
            for d in &node.summary.decisions {
                out.push_str(&format!("      <decision>{}</decision>\n", xml_escape(d)));
            }
            out.push_str("    </decisions>\n");
        }

        if !node.summary.rejected_approaches.is_empty() {
            out.push_str("    <rejected_approaches>\n");
            for r in &node.summary.rejected_approaches {
                out.push_str("      <approach>\n");
                out.push_str(&format!(
                    "        <description>{}</description>\n",
                    xml_escape(&r.description)
                ));
                out.push_str(&format!(
                    "        <reason>{}</reason>\n",
                    xml_escape(&r.reason)
                ));
                out.push_str("      </approach>\n");
            }
            out.push_str("    </rejected_approaches>\n");
        }

        if !node.summary.open_threads.is_empty() {
            out.push_str("    <open_threads>\n");
            for t in &node.summary.open_threads {
                out.push_str(&format!("      <thread>{}</thread>\n", xml_escape(t)));
            }
            out.push_str("    </open_threads>\n");
        }

        if !node.summary.key_artifacts.is_empty() {
            out.push_str("    <key_artifacts>\n");
            for a in &node.summary.key_artifacts {
                out.push_str(&format!("      <artifact>{}</artifact>\n", xml_escape(a)));
            }
            out.push_str("    </key_artifacts>\n");
        }

        out.push_str("  </current_node_context>\n");
    }

    out.push_str("</memex_context>\n");
    Ok(out)
}

fn generate_plain(path: &[Uuid], node_map: &HashMap<Uuid, &ConversationNode>) -> Result<String> {
    let mut out = String::new();

    if path.is_empty() {
        return Ok(out);
    }

    let root_id = path[0];
    if let Some(root) = node_map.get(&root_id) {
        out.push_str("PROJECT CONTEXT\n");
        out.push_str(&format!("Goal: {}\n", root.summary.goal));
        if !root.summary.decisions.is_empty() {
            out.push_str("Key Decisions:\n");
            for d in &root.summary.decisions {
                out.push_str(&format!("  - {}\n", d));
            }
        }
        out.push('\n');
    }

    let ancestor_range = if path.len() > 2 {
        &path[1..path.len() - 1]
    } else {
        &[]
    };
    if !ancestor_range.is_empty() {
        out.push_str("ANCESTOR CONTEXT\n");
        for &anc_id in ancestor_range {
            if let Some(node) = node_map.get(&anc_id) {
                let short = &node.id.to_string()[..8];
                out.push_str(&format!("[{}] Goal: {}", short, node.summary.goal));
                if !node.summary.decisions.is_empty() {
                    out.push_str(&format!(
                        " | Decisions: {}",
                        node.summary.decisions.join("; ")
                    ));
                }
                out.push('\n');
            }
        }
        out.push('\n');
    }

    let immediate = path[path.len() - 1];
    if let Some(node) = node_map.get(&immediate) {
        out.push_str("CURRENT NODE CONTEXT\n");
        out.push_str(&format!("Goal: {}\n", node.summary.goal));

        if !node.summary.decisions.is_empty() {
            out.push_str("Decisions:\n");
            for d in &node.summary.decisions {
                out.push_str(&format!("  - {}\n", d));
            }
        }

        if !node.summary.rejected_approaches.is_empty() {
            out.push_str("Rejected:\n");
            for r in &node.summary.rejected_approaches {
                out.push_str(&format!("  - {} -- {}\n", r.description, r.reason));
            }
        }

        if !node.summary.open_threads.is_empty() {
            out.push_str("Open Threads:\n");
            for t in &node.summary.open_threads {
                out.push_str(&format!("  - {}\n", t));
            }
        }

        if !node.summary.key_artifacts.is_empty() {
            out.push_str(&format!(
                "Key Artifacts: {}\n",
                node.summary.key_artifacts.join(", ")
            ));
        }
    }

    Ok(out)
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{NodeStatus, NodeSummary};
    use chrono::Utc;

    fn make_node(id: Uuid, parent_ids: Vec<Uuid>) -> ConversationNode {
        ConversationNode {
            id,
            parent_ids,
            git_ref: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            summary: NodeSummary {
                goal: format!("Goal {}", &id.to_string()[..8]),
                ..Default::default()
            },
            raw_transcript_ref: None,
            tags: vec![],
            status: NodeStatus::Active,
        }
    }

    fn node_map(nodes: &[ConversationNode]) -> HashMap<Uuid, &ConversationNode> {
        nodes.iter().map(|n| (n.id, n)).collect()
    }

    // --- find_path ---

    #[test]
    fn find_path_start_equals_target() {
        let id = Uuid::new_v4();
        let n = make_node(id, vec![]);
        let nodes = vec![n];
        let map = node_map(&nodes);
        assert_eq!(find_path(id, id, &map), Some(vec![id]));
    }

    #[test]
    fn find_path_direct_parent_child() {
        let root_id = Uuid::new_v4();
        let child_id = Uuid::new_v4();
        let root = make_node(root_id, vec![]);
        let child = make_node(child_id, vec![root_id]);
        let nodes = vec![root, child];
        let map = node_map(&nodes);
        assert_eq!(
            find_path(root_id, child_id, &map),
            Some(vec![root_id, child_id])
        );
    }

    #[test]
    fn find_path_multi_hop() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();
        let na = make_node(a, vec![]);
        let nb = make_node(b, vec![a]);
        let nc = make_node(c, vec![b]);
        let nodes = vec![na, nb, nc];
        let map = node_map(&nodes);
        assert_eq!(find_path(a, c, &map), Some(vec![a, b, c]));
    }

    #[test]
    fn find_path_branching_dag() {
        let root = Uuid::new_v4();
        let b1 = Uuid::new_v4();
        let b2 = Uuid::new_v4();
        let g = Uuid::new_v4();
        let n_root = make_node(root, vec![]);
        let n_b1 = make_node(b1, vec![root]);
        let n_b2 = make_node(b2, vec![root]);
        let n_g = make_node(g, vec![b2]);
        let nodes = vec![n_root, n_b1, n_b2, n_g];
        let map = node_map(&nodes);
        let path = find_path(root, g, &map).unwrap();
        assert_eq!(path[0], root);
        assert_eq!(*path.last().unwrap(), g);
        assert!(path.contains(&b2));
        assert!(!path.contains(&b1));
    }

    #[test]
    fn find_path_unreachable_target_returns_none() {
        let start = Uuid::new_v4();
        let target = Uuid::new_v4();
        let n_start = make_node(start, vec![]);
        let n_target = make_node(target, vec![]); // no relationship to start
        let nodes = vec![n_start, n_target];
        let map = node_map(&nodes);
        assert_eq!(find_path(start, target, &map), None);
    }

    // --- trim_path ---

    #[test]
    fn trim_path_empty() {
        let result: Vec<Uuid> = trim_path(vec![], 5);
        assert!(result.is_empty());
    }

    #[test]
    fn trim_path_single_node() {
        let a = Uuid::new_v4();
        assert_eq!(trim_path(vec![a], 2), vec![a]);
    }

    #[test]
    fn trim_path_two_nodes_any_depth() {
        let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
        // Early return fires regardless of depth
        assert_eq!(trim_path(vec![a, b], 0), vec![a, b]);
        assert_eq!(trim_path(vec![a, b], 99), vec![a, b]);
    }

    #[test]
    fn trim_path_within_depth() {
        let (r, a1, a2, t) = (
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
        );
        let path = vec![r, a1, a2, t];
        // 2 ancestors, depth=3 → no trimming
        assert_eq!(trim_path(path.clone(), 3), path);
    }

    #[test]
    fn trim_path_truncates_oldest() {
        let (r, a1, a2, a3, t) = (
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
        );
        let path = vec![r, a1, a2, a3, t];
        // 3 ancestors, depth=2 → skip oldest (a1), keep [a2, a3]
        assert_eq!(trim_path(path, 2), vec![r, a2, a3, t]);
    }

    #[test]
    fn trim_path_depth_zero() {
        let (r, a1, a2, t) = (
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
        );
        let path = vec![r, a1, a2, t];
        // depth=0 → no ancestors kept, just root and target
        assert_eq!(trim_path(path, 0), vec![r, t]);
    }

    // --- OutputFormat ---

    #[test]
    fn output_format_from_str_variants() {
        assert_eq!(
            OutputFormat::from_str("markdown").unwrap(),
            OutputFormat::Markdown
        );
        assert_eq!(
            OutputFormat::from_str("md").unwrap(),
            OutputFormat::Markdown
        );
        assert_eq!(
            OutputFormat::from_str("MARKDOWN").unwrap(),
            OutputFormat::Markdown
        );
        assert_eq!(OutputFormat::from_str("xml").unwrap(), OutputFormat::Xml);
        assert_eq!(
            OutputFormat::from_str("plain").unwrap(),
            OutputFormat::Plain
        );
        assert_eq!(OutputFormat::from_str("text").unwrap(), OutputFormat::Plain);
    }

    #[test]
    fn output_format_from_str_invalid() {
        let result = OutputFormat::from_str("json");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown format"));
    }
}

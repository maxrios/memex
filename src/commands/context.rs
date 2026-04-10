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

pub fn run(id: Option<&str>, format: OutputFormat) -> Result<()> {
    let store = GraphStore::open_from_cwd()?;
    let graph = store.load_graph()?;
    let node_id = store.resolve_node_id(id)?;
    let nodes = store.load_all_nodes()?;
    let node_map: HashMap<Uuid, &ConversationNode> = nodes.iter().map(|n| (n.id, n)).collect();

    // Build parent path: from root to target node
    let root_id = graph
        .root_id
        .ok_or_else(|| anyhow::anyhow!("Graph has no root node"))?;

    let path = find_path(root_id, node_id, &node_map)
        .ok_or_else(|| anyhow::anyhow!("Could not find path from root to node {}", node_id))?;

    let output = match format {
        OutputFormat::Markdown => generate_markdown(&path, &node_map, node_id)?,
        OutputFormat::Xml => generate_xml(&path, &node_map, node_id)?,
        OutputFormat::Plain => generate_plain(&path, &node_map, node_id)?,
    };

    println!("{}", output);
    Ok(())
}

/// Find the path (list of node IDs) from `start` to `target` using BFS.
fn find_path(
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

    // If not found via parent traversal, just return [target] as fallback
    Some(vec![target])
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

fn generate_xml(
    path: &[Uuid],
    node_map: &HashMap<Uuid, &ConversationNode>,
    _target_id: Uuid,
) -> Result<String> {
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

fn generate_plain(
    path: &[Uuid],
    node_map: &HashMap<Uuid, &ConversationNode>,
    _target_id: Uuid,
) -> Result<String> {
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

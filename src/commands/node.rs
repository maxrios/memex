use anyhow::{bail, Context, Result};
use chrono::Utc;

use crate::editor;
use crate::git;
use crate::models::{ConversationNode, NodeStatus, NodeSummary};
use crate::store::GraphStore;

pub fn create(
    parent: Option<&str>,
    git_ref: Option<&str>,
    tags: &[String],
    goal: Option<&str>,
) -> Result<()> {
    let store = GraphStore::open_from_cwd()?;
    let mut graph = store.load_graph()?;

    // Resolve parent ID
    let parent_id = if let Some(p) = parent {
        Some(store.find_node_id_by_prefix(p)?)
    } else {
        store.get_active_id()?
    };

    let parent_ids = parent_id.map(|id| vec![id]).unwrap_or_default();

    // Detect git ref if not provided
    let resolved_git_ref = if let Some(r) = git_ref {
        Some(r.to_string())
    } else if git::is_git_repo() {
        git::detect_git_ref()
    } else {
        None
    };

    // Use --goal directly or open editor
    let summary = if let Some(g) = goal {
        use crate::models::NodeSummary;
        NodeSummary {
            goal: g.to_string(),
            decisions: Vec::new(),
            rejected_approaches: Vec::new(),
            open_threads: Vec::new(),
            key_artifacts: Vec::new(),
        }
    } else {
        println!("Opening editor to fill in node summary...");
        editor::edit_node_summary(None)?
    };

    let mut node = ConversationNode::new(parent_ids.clone(), resolved_git_ref, tags.to_vec());
    node.summary = summary;

    let node_id = node.id;

    // Add edges to graph
    for pid in &parent_ids {
        graph.add_edge(*pid, node_id);
    }

    store.save_node(&node)?;
    store.save_graph(&graph)?;
    store.set_active_id(node_id)?;

    println!("Created node: {}", node_id);
    println!("Active node set to: {}", node.short_id());

    Ok(())
}

pub fn edit(
    id: Option<&str>,
    summary_toml: Option<&str>,
    goal: Option<&str>,
    decisions: &[String],
    artifacts: &[String],
    open_threads: &[String],
    rejected: &[String],
) -> Result<()> {
    use crate::models::{RejectedApproach, NodeSummaryToml};

    let has_additive = goal.is_some()
        || !decisions.is_empty()
        || !artifacts.is_empty()
        || !open_threads.is_empty()
        || !rejected.is_empty();

    if summary_toml.is_some() && has_additive {
        anyhow::bail!(
            "--summary cannot be combined with --goal, --decision, --artifact, --open-thread, or --rejected"
        );
    }

    let store = GraphStore::open_from_cwd()?;
    let node_id = store.resolve_node_id(id)?;
    let mut node = store.load_node(node_id)?;

    if let Some(toml_str) = summary_toml {
        let parsed: NodeSummaryToml =
            toml::from_str(toml_str).context("Failed to parse --summary TOML")?;
        node.summary = NodeSummary::from(parsed);
    } else if has_additive {
        if let Some(g) = goal {
            if g.is_empty() {
                anyhow::bail!("--goal cannot be empty");
            }
            node.summary.goal = g.to_string();
        }
        node.summary.decisions.extend_from_slice(decisions);
        node.summary.key_artifacts.extend_from_slice(artifacts);
        node.summary.open_threads.extend_from_slice(open_threads);
        for val in rejected {
            let approach: RejectedApproach = toml::from_str(val)
                .context("Failed to parse --rejected TOML (expected: description = \"...\" and reason = \"...\")")?;
            node.summary.rejected_approaches.push(approach);
        }
    } else {
        println!("Opening editor to edit node {}...", node.short_id());
        node.summary = editor::edit_node_summary(Some(&node.summary))?;
    }

    node.updated_at = Utc::now();
    store.save_node(&node)?;

    println!("Node {} updated.", node.short_id());
    Ok(())
}

pub fn show(id: Option<&str>) -> Result<()> {
    let store = GraphStore::open_from_cwd()?;
    let state = store.load_state()?;
    let node_id = store.resolve_node_id(id)?;
    let node = store.load_node(node_id)?;

    let is_active = state.active_id == Some(node.id);
    print_node_detail(&node, is_active);
    Ok(())
}

pub fn list() -> Result<()> {
    let store = GraphStore::open_from_cwd()?;
    let state = store.load_state()?;
    let nodes = store.load_all_nodes()?;

    if nodes.is_empty() {
        println!("No nodes found.");
        return Ok(());
    }

    // Header
    println!(
        "{:<10} {:<10} {:<10} {:<20} {:<52} Created",
        "ID", "Parent", "Status", "GitRef", "Goal"
    );
    println!("{}", "-".repeat(120));

    for node in &nodes {
        let active_marker = if state.active_id == Some(node.id) {
            "*"
        } else {
            " "
        };
        let short_id = format!("{}{}", active_marker, node.short_id());
        let parent = node
            .parent_ids
            .first()
            .map(|id| id.to_string()[..8].to_string())
            .unwrap_or_else(|| "-".to_string());
        let status = format!("{}", node.status);
        let git_ref = node
            .git_ref
            .as_deref()
            .unwrap_or("-")
            .chars()
            .take(18)
            .collect::<String>();
        let goal = node.summary.goal.chars().take(50).collect::<String>();
        let goal = if node.summary.goal.len() > 50 {
            format!("{}…", goal)
        } else {
            goal
        };
        let created = node.created_at.format("%Y-%m-%d %H:%M").to_string();

        println!(
            "{:<10} {:<10} {:<10} {:<20} {:<52} {}",
            short_id, parent, status, git_ref, goal, created
        );
    }

    println!("\n* = active node");
    Ok(())
}

pub fn set_status(id: Option<&str>, status: NodeStatus) -> Result<()> {
    let store = GraphStore::open_from_cwd()?;
    let node_id = store.resolve_node_id(id)?;
    let mut node = store.load_node(node_id)?;

    let verb = match &status {
        NodeStatus::Resolved => "resolved",
        NodeStatus::Abandoned => "abandoned",
        NodeStatus::Active => "reopened",
    };

    // Validate transitions
    match (&node.status, &status) {
        (NodeStatus::Active, NodeStatus::Active) => {
            bail!("Node is already active.");
        }
        (NodeStatus::Resolved, NodeStatus::Resolved) => {
            bail!("Node is already resolved.");
        }
        (NodeStatus::Abandoned, NodeStatus::Abandoned) => {
            bail!("Node is already abandoned.");
        }
        _ => {}
    }

    node.status = status;
    node.updated_at = Utc::now();
    store.save_node(&node)?;

    println!("Node {} {}.", node.short_id(), verb);
    Ok(())
}

fn print_node_detail(node: &ConversationNode, is_active: bool) {
    let active_str = if is_active { " [ACTIVE]" } else { "" };
    println!("┌─ Node: {}{}", node.id, active_str);
    println!("│  Status:  {}", node.status);
    if let Some(ref git_ref) = node.git_ref {
        println!("│  GitRef:  {}", git_ref);
    }
    println!(
        "│  Created: {}",
        node.created_at.format("%Y-%m-%d %H:%M:%S UTC")
    );
    println!(
        "│  Updated: {}",
        node.updated_at.format("%Y-%m-%d %H:%M:%S UTC")
    );
    if !node.tags.is_empty() {
        println!("│  Tags:    {}", node.tags.join(", "));
    }
    if !node.parent_ids.is_empty() {
        let parents: Vec<String> = node
            .parent_ids
            .iter()
            .map(|id| id.to_string()[..8].to_string())
            .collect();
        println!("│  Parents: {}", parents.join(", "));
    }
    println!("│");
    println!("│  Goal:");
    println!("│    {}", node.summary.goal);

    if !node.summary.decisions.is_empty() {
        println!("│");
        println!("│  Decisions:");
        for d in &node.summary.decisions {
            println!("│    • {}", d);
        }
    }

    if !node.summary.rejected_approaches.is_empty() {
        println!("│");
        println!("│  Rejected Approaches:");
        for r in &node.summary.rejected_approaches {
            println!("│    ✗ {} — {}", r.description, r.reason);
        }
    }

    if !node.summary.open_threads.is_empty() {
        println!("│");
        println!("│  Open Threads:");
        for t in &node.summary.open_threads {
            println!("│    ? {}", t);
        }
    }

    if !node.summary.key_artifacts.is_empty() {
        println!("│");
        println!("│  Key Artifacts:");
        for a in &node.summary.key_artifacts {
            println!("│    ◆ {}", a);
        }
    }

    println!("└{}", "─".repeat(60));
}

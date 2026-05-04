use std::collections::HashMap;

use anyhow::Result;
use uuid::Uuid;

use crate::models::ConversationNode;
use crate::store::GraphStore;

pub fn view() -> Result<()> {
    let store = GraphStore::open_from_cwd()?;
    let state = store.load_state()?;
    let nodes = store.load_all_nodes()?;

    if nodes.is_empty() {
        println!("No nodes in graph.");
        return Ok(());
    }

    let node_map: HashMap<Uuid, &ConversationNode> = nodes.iter().map(|n| (n.id, n)).collect();

    // Build children map by inverting each node's parent_ids
    let mut children: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for node in &nodes {
        for &parent_id in &node.parent_ids {
            children.entry(parent_id).or_default().push(node.id);
        }
    }

    // Roots are nodes with no parents. `load_all_nodes` already sorts by
    // created_at, so multiple-root output is stable across runs.
    let roots: Vec<Uuid> = nodes
        .iter()
        .filter(|n| n.parent_ids.is_empty())
        .map(|n| n.id)
        .collect();

    println!("Conversation Graph");
    println!("{}", "═".repeat(50));

    for (i, root) in roots.iter().enumerate() {
        let is_last = i == roots.len() - 1;
        print_subtree(*root, &node_map, &children, state.active_id, "", is_last);
    }

    println!();
    println!("Legend: ● Active  ✓ Resolved  ✗ Abandoned  [*] = current active node");

    Ok(())
}

fn print_subtree(
    node_id: Uuid,
    node_map: &HashMap<Uuid, &ConversationNode>,
    children: &HashMap<Uuid, Vec<Uuid>>,
    active_id: Option<Uuid>,
    prefix: &str,
    is_last: bool,
) {
    let connector = if is_last { "└── " } else { "├── " };

    if let Some(node) = node_map.get(&node_id) {
        let short_id = &node.id.to_string()[..8];
        let status_icon = node.status_icon();

        let active_marker = if active_id == Some(node.id) {
            " [*]"
        } else {
            ""
        };

        let goal = node.summary.goal.chars().take(45).collect::<String>();
        let goal = if node.summary.goal.len() > 45 {
            format!("{}…", goal)
        } else {
            goal
        };

        println!(
            "{}{}{} {} {}{}",
            prefix, connector, short_id, status_icon, goal, active_marker
        );

        let child_prefix = format!("{}{}", prefix, if is_last { "    " } else { "│   " });

        let child_ids = children.get(&node_id).cloned().unwrap_or_default();
        for (i, child_id) in child_ids.iter().enumerate() {
            let child_is_last = i == child_ids.len() - 1;
            print_subtree(
                *child_id,
                node_map,
                children,
                active_id,
                &child_prefix,
                child_is_last,
            );
        }
    } else {
        // Node referenced in edges but not found
        let connector = if is_last { "└── " } else { "├── " };
        let short_id = &node_id.to_string()[..8];
        println!("{}{}{}  <missing>", prefix, connector, short_id);
    }
}

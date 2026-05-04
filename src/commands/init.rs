use std::io::{self, IsTerminal, Read};

use anyhow::{Context, Result};

use crate::editor;
use crate::git;
use crate::models::{ConversationNode, NodeSummary, State};
use crate::store::GraphStore;

pub fn run() -> Result<()> {
    let cwd = std::env::current_dir().context("Failed to get current directory")?;
    let store = GraphStore::open(cwd);

    if store.is_initialized() {
        eprintln!("Warning: .memex/ already exists. Skipping initialization.");
        return Ok(());
    }

    store.initialize()?;

    println!("Initialized .memex/ directory.");

    // Detect git context
    let git_ref = if git::is_git_repo() {
        let r = git::detect_git_ref();
        if let Some(ref git_str) = r {
            println!("Detected git ref: {}", git_str);
        }
        r
    } else {
        None
    };

    // Get project goal from user
    let goal = get_project_goal()?;

    // Create root node
    let mut root_node = ConversationNode::new(vec![], git_ref, vec![]);
    root_node.summary = NodeSummary {
        goal,
        decisions: Vec::new(),
        rejected_approaches: Vec::new(),
        open_threads: Vec::new(),
        key_artifacts: Vec::new(),
    };

    let root_id = root_node.id;
    store.save_node(&root_node)?;

    // Set active node. The root is identified at read time as the unique node
    // with empty parent_ids, so no separate graph file is needed.
    let state = State {
        active_id: Some(root_id),
    };
    store.save_state(&state)?;

    println!("\nCreated root node: {}", root_id);
    println!("Active node set to root.");
    println!("\nTip: Use `memex node create` to start a new conversation node.");

    Ok(())
}

fn get_project_goal() -> Result<String> {
    let stdin = io::stdin();
    if stdin.is_terminal() {
        // Interactive: open editor
        println!("Opening editor to describe your project goal...");
        let initial = "# Describe your project goal below (this line will be ignored)\n# Save and close to continue.\n\n";
        let edited = editor::edit_text(initial, ".md")?;
        // Strip comment lines
        let goal: String = edited
            .lines()
            .filter(|l| !l.trim_start().starts_with('#'))
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string();
        if goal.is_empty() {
            Ok("Project root node".to_string())
        } else {
            Ok(goal)
        }
    } else {
        // Non-interactive: read from stdin
        let mut input = String::new();
        stdin
            .lock()
            .read_to_string(&mut input)
            .context("Failed to read from stdin")?;
        let goal = input.trim().to_string();
        if goal.is_empty() {
            Ok("Project root node".to_string())
        } else {
            Ok(goal)
        }
    }
}

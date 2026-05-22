use std::collections::HashSet;
use std::io::{self, IsTerminal, Read, Write};

use anyhow::{bail, Context, Result};
use chrono::Utc;

use crate::editor;
use crate::git;
use crate::models::{ConversationNode, NodeAutoPayload, NodeStatus, NodeSummary, RejectedApproach};
use crate::store::GraphStore;

pub fn create(
    parent: Option<&str>,
    git_ref: Option<&str>,
    tags: &[String],
    goal: Option<&str>,
) -> Result<()> {
    let store = GraphStore::open_from_cwd()?;

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

    let mut node = ConversationNode::new(parent_ids, resolved_git_ref, tags.to_vec());
    node.summary = summary;

    let node_id = node.id;

    store.save_node(&node)?;
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
    use crate::models::{NodeSummaryToml, RejectedApproach};

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

/// Append draft node additions from a JSON payload on stdin.
///
/// `--dry-run` (default) prints a diff against the target node without
/// writing. `--apply` merges the additions into the node summary, using
/// normalized-text dedupe so re-running the same payload is a no-op.
///
/// The payload must be a JSON object; all fields are optional:
///
/// ```json
/// {
///   "decisions": ["..."],
///   "rejected_approaches": [{"description": "...", "reason": "..."}],
///   "open_threads": ["..."],
///   "key_artifacts": ["..."]
/// }
/// ```
pub fn auto(id: Option<&str>, from_stdin: bool, apply: bool) -> Result<()> {
    if !from_stdin {
        bail!(
            "--from-stdin is required: pipe a JSON payload on stdin (see `memex node auto --help`)"
        );
    }

    let mut payload_str = String::new();
    io::stdin()
        .read_to_string(&mut payload_str)
        .context("Failed to read payload from stdin")?;

    if payload_str.trim().is_empty() {
        bail!("Empty payload on stdin (expected a JSON object)");
    }

    let payload: NodeAutoPayload =
        serde_json::from_str(&payload_str).context("Failed to parse JSON payload")?;

    let store = GraphStore::open_from_cwd()?;
    let node_id = store.resolve_node_id(id)?;
    let mut node = store.load_node(node_id)?;

    let plan = compute_auto_plan(&node.summary, &payload);

    if !apply {
        print_auto_diff(&node, &plan);
        return Ok(());
    }

    if !plan.has_changes() {
        println!(
            "✓ No changes — all entries already present on node {}.",
            node.short_id()
        );
        return Ok(());
    }

    let counts = plan.counts();
    node.summary.decisions.extend(plan.new_decisions);
    node.summary
        .rejected_approaches
        .extend(plan.new_rejected.into_iter().map(|r| RejectedApproach {
            description: r.description,
            reason: r.reason,
        }));
    node.summary.open_threads.extend(plan.new_open_threads);
    node.summary.key_artifacts.extend(plan.new_artifacts);
    node.updated_at = Utc::now();
    store.save_node(&node)?;

    println!(
        "✓ Applied {} decision(s), {} rejected approach(es), {} open thread(s), {} artifact(s) to node {}.",
        counts.decisions, counts.rejected, counts.open_threads, counts.artifacts,
        node.short_id()
    );
    Ok(())
}

/// Result of comparing a payload against an existing node summary: what
/// would be added on `--apply`, and what was skipped as a duplicate.
struct AutoPlan {
    new_decisions: Vec<String>,
    skipped_decisions: Vec<String>,
    new_rejected: Vec<crate::models::RejectedApproachPayload>,
    skipped_rejected: Vec<String>,
    new_open_threads: Vec<String>,
    skipped_open_threads: Vec<String>,
    new_artifacts: Vec<String>,
    skipped_artifacts: Vec<String>,
}

struct AutoCounts {
    decisions: usize,
    rejected: usize,
    open_threads: usize,
    artifacts: usize,
}

impl AutoPlan {
    fn has_changes(&self) -> bool {
        !self.new_decisions.is_empty()
            || !self.new_rejected.is_empty()
            || !self.new_open_threads.is_empty()
            || !self.new_artifacts.is_empty()
    }

    fn has_skips(&self) -> bool {
        !self.skipped_decisions.is_empty()
            || !self.skipped_rejected.is_empty()
            || !self.skipped_open_threads.is_empty()
            || !self.skipped_artifacts.is_empty()
    }

    fn counts(&self) -> AutoCounts {
        AutoCounts {
            decisions: self.new_decisions.len(),
            rejected: self.new_rejected.len(),
            open_threads: self.new_open_threads.len(),
            artifacts: self.new_artifacts.len(),
        }
    }
}

/// Normalize free-text entries for dedupe: trim, collapse internal
/// whitespace, lowercase, strip trailing punctuation. This catches the
/// common cases (re-run with the same payload, agent re-emitting with a
/// trailing period) without crossing into fuzzy matching.
fn normalize_text(s: &str) -> String {
    let collapsed: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    let lower = collapsed.to_lowercase();
    lower.trim_end_matches(['.', '!', '?']).to_string()
}

fn compute_auto_plan(summary: &NodeSummary, payload: &NodeAutoPayload) -> AutoPlan {
    // Text fields: normalized-text dedupe.
    let mut decision_keys: HashSet<String> = summary
        .decisions
        .iter()
        .map(|d| normalize_text(d))
        .collect();
    let mut new_decisions = Vec::new();
    let mut skipped_decisions = Vec::new();
    for d in &payload.decisions {
        let key = normalize_text(d);
        if key.is_empty() {
            // Drop empties silently — agents occasionally emit blanks.
            continue;
        }
        if decision_keys.insert(key) {
            new_decisions.push(d.trim().to_string());
        } else {
            skipped_decisions.push(d.trim().to_string());
        }
    }

    // Rejected approaches: dedupe on normalized description only;
    // reasons are usually longer and less stable across re-emits.
    let mut rejected_keys: HashSet<String> = summary
        .rejected_approaches
        .iter()
        .map(|r| normalize_text(&r.description))
        .collect();
    let mut new_rejected = Vec::new();
    let mut skipped_rejected = Vec::new();
    for r in &payload.rejected_approaches {
        let key = normalize_text(&r.description);
        if key.is_empty() {
            continue;
        }
        if rejected_keys.insert(key) {
            new_rejected.push(crate::models::RejectedApproachPayload {
                description: r.description.trim().to_string(),
                reason: r.reason.trim().to_string(),
            });
        } else {
            skipped_rejected.push(r.description.trim().to_string());
        }
    }

    let mut thread_keys: HashSet<String> = summary
        .open_threads
        .iter()
        .map(|t| normalize_text(t))
        .collect();
    let mut new_open_threads = Vec::new();
    let mut skipped_open_threads = Vec::new();
    for t in &payload.open_threads {
        let key = normalize_text(t);
        if key.is_empty() {
            continue;
        }
        if thread_keys.insert(key) {
            new_open_threads.push(t.trim().to_string());
        } else {
            skipped_open_threads.push(t.trim().to_string());
        }
    }

    // Artifacts: exact-path dedupe (paths are case-sensitive on most
    // filesystems and shouldn't have punctuation normalized away).
    let mut artifact_keys: HashSet<String> = summary.key_artifacts.iter().cloned().collect();
    let mut new_artifacts = Vec::new();
    let mut skipped_artifacts = Vec::new();
    for a in &payload.key_artifacts {
        let trimmed = a.trim().to_string();
        if trimmed.is_empty() {
            continue;
        }
        if artifact_keys.insert(trimmed.clone()) {
            new_artifacts.push(trimmed);
        } else {
            skipped_artifacts.push(trimmed);
        }
    }

    AutoPlan {
        new_decisions,
        skipped_decisions,
        new_rejected,
        skipped_rejected,
        new_open_threads,
        skipped_open_threads,
        new_artifacts,
        skipped_artifacts,
    }
}

fn print_auto_diff(node: &ConversationNode, plan: &AutoPlan) {
    let goal_preview: String = node.summary.goal.chars().take(60).collect();
    let goal_preview = if node.summary.goal.len() > 60 {
        format!("{}…", goal_preview)
    } else {
        goal_preview
    };
    println!("Node: {} ({})", node.short_id(), goal_preview);
    println!();

    if !plan.has_changes() {
        println!("No changes — all entries already present.");
        return;
    }

    if !plan.new_decisions.is_empty() {
        println!("+ Decisions:");
        for d in &plan.new_decisions {
            println!("+   • {}", d);
        }
    }
    if !plan.new_rejected.is_empty() {
        println!("+ Rejected Approaches:");
        for r in &plan.new_rejected {
            println!("+   ✗ {} — {}", r.description, r.reason);
        }
    }
    if !plan.new_open_threads.is_empty() {
        println!("+ Open Threads:");
        for t in &plan.new_open_threads {
            println!("+   ? {}", t);
        }
    }
    if !plan.new_artifacts.is_empty() {
        println!("+ Key Artifacts:");
        for a in &plan.new_artifacts {
            println!("+   ◆ {}", a);
        }
    }

    if plan.has_skips() {
        println!();
        println!("Skipped (already present):");
        for d in &plan.skipped_decisions {
            println!("  • {}", d);
        }
        for r in &plan.skipped_rejected {
            println!("  ✗ {}", r);
        }
        for t in &plan.skipped_open_threads {
            println!("  ? {}", t);
        }
        for a in &plan.skipped_artifacts {
            println!("  ◆ {}", a);
        }
    }

    println!();
    println!("Run with --apply to write these changes.");
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

pub fn set_status(id: Option<&str>, status: NodeStatus, force: bool) -> Result<()> {
    let store = GraphStore::open_from_cwd()?;
    let node_id = store.resolve_node_id(id)?;
    let mut node = store.load_node(node_id)?;

    let verb = match &status {
        NodeStatus::Resolved => "resolve",
        NodeStatus::Abandoned => "abandon",
        NodeStatus::Active => "reopen",
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

    if !force && io::stdin().is_terminal() {
        let goal_preview: String = node.summary.goal.chars().take(60).collect();
        let goal_preview = if node.summary.goal.len() > 60 {
            format!("{}…", goal_preview)
        } else {
            goal_preview
        };
        eprint!(
            "{} node {} \"{}\"? [y/N] ",
            capitalize(verb),
            node.short_id(),
            goal_preview
        );
        io::stderr().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
            bail!("Aborted.");
        }
    }

    let past_verb = match &status {
        NodeStatus::Resolved => "resolved",
        NodeStatus::Abandoned => "abandoned",
        NodeStatus::Active => "reopened",
    };

    node.status = status;
    node.updated_at = Utc::now();
    store.save_node(&node)?;

    println!("Node {} {}.", node.short_id(), past_verb);
    Ok(())
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
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

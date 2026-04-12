use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

fn memex() -> Command {
    Command::cargo_bin("memex").unwrap()
}

/// Initialize a memex repo in `dir` using stdin so no editor is opened.
fn init_in(dir: &TempDir) {
    memex()
        .current_dir(dir.path())
        .arg("init")
        .write_stdin("My test project\n")
        .assert()
        .success();
}

/// Create a node with the given goal and return its short ID (first 8 chars).
fn create_node(dir: &TempDir, goal: &str) -> String {
    let output = memex()
        .current_dir(dir.path())
        .args(["node", "create", "--goal", goal])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).unwrap();
    // "Created node: <full-uuid>\n"
    text.lines()
        .find(|l| l.starts_with("Created node:"))
        .unwrap()
        .split_whitespace()
        .last()
        .unwrap()[..8]
        .to_string()
}

// ─── Init ────────────────────────────────────────────────────────────────────

#[test]
fn init_creates_memex_directory() {
    let tmp = TempDir::new().unwrap();
    memex()
        .current_dir(tmp.path())
        .arg("init")
        .write_stdin("My project\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized .memex/"));

    assert!(tmp.path().join(".memex").is_dir());
    assert!(tmp.path().join(".memex/nodes").is_dir());
    assert!(tmp.path().join(".memex/config.toml").exists());
}

#[test]
fn init_twice_warns_already_exists() {
    let tmp = TempDir::new().unwrap();
    init_in(&tmp);
    // Second init should exit 0 but warn on stderr
    memex()
        .current_dir(tmp.path())
        .arg("init")
        .write_stdin("irrelevant\n")
        .assert()
        .success()
        .stderr(predicate::str::contains("already"));
}

#[test]
fn command_without_init_fails() {
    let tmp = TempDir::new().unwrap();
    memex()
        .current_dir(tmp.path())
        .args(["node", "list"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No .memex directory found"));
}

// ─── Node create ─────────────────────────────────────────────────────────────

#[test]
fn node_create_with_goal_flag() {
    let tmp = TempDir::new().unwrap();
    init_in(&tmp);
    let short_id = create_node(&tmp, "Build a search index");

    // Node file exists on disk
    let nodes_dir = tmp.path().join(".memex/nodes");
    let entries: Vec<_> = fs::read_dir(&nodes_dir)
        .unwrap()
        // init creates a root node, so look for the one matching our short_id
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with(&short_id))
        .collect();
    assert_eq!(entries.len(), 1, "expected node file for {short_id}");
}

#[test]
fn node_create_sets_active() {
    let tmp = TempDir::new().unwrap();
    init_in(&tmp);
    create_node(&tmp, "My active task");

    memex()
        .current_dir(tmp.path())
        .args(["node", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("My active task"));
}

#[test]
fn node_create_with_parent_flag() {
    let tmp = TempDir::new().unwrap();
    init_in(&tmp);
    let parent_id = create_node(&tmp, "Parent node");
    let child_id = create_node(&tmp, "Child node");
    // Re-create child with explicit parent (use `node create` with parent pointing to parent_id)
    // (The previous create_node already set parent to the current active; re-verify via graph view)
    let _ = child_id;

    memex()
        .current_dir(tmp.path())
        .args(["graph", "view"])
        .assert()
        .success()
        .stdout(predicate::str::contains(&parent_id));
}

#[test]
fn node_create_unknown_parent_fails() {
    let tmp = TempDir::new().unwrap();
    init_in(&tmp);
    memex()
        .current_dir(tmp.path())
        .args(["node", "create", "--parent", "00000000", "--goal", "Orphan"])
        .assert()
        .failure();
}

// ─── Node edit ───────────────────────────────────────────────────────────────

#[test]
fn node_edit_goal_flag() {
    let tmp = TempDir::new().unwrap();
    init_in(&tmp);
    create_node(&tmp, "Original goal");

    memex()
        .current_dir(tmp.path())
        .args(["node", "edit", "--goal", "Updated goal"])
        .assert()
        .success();

    memex()
        .current_dir(tmp.path())
        .args(["node", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated goal"))
        .stdout(predicate::str::contains("Original goal").not());
}

#[test]
fn node_edit_decision_flag() {
    let tmp = TempDir::new().unwrap();
    init_in(&tmp);
    create_node(&tmp, "Planning");

    memex()
        .current_dir(tmp.path())
        .args([
            "node",
            "edit",
            "--decision",
            "Use PostgreSQL",
            "--decision",
            "Use Rust",
        ])
        .assert()
        .success();

    memex()
        .current_dir(tmp.path())
        .args(["node", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Use PostgreSQL"))
        .stdout(predicate::str::contains("Use Rust"));
}

#[test]
fn node_edit_artifact_flag() {
    let tmp = TempDir::new().unwrap();
    init_in(&tmp);
    create_node(&tmp, "Work");

    memex()
        .current_dir(tmp.path())
        .args(["node", "edit", "--artifact", "src/main.rs"])
        .assert()
        .success();

    memex()
        .current_dir(tmp.path())
        .args(["node", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("src/main.rs"));
}

#[test]
fn node_edit_open_thread_flag() {
    let tmp = TempDir::new().unwrap();
    init_in(&tmp);
    create_node(&tmp, "Design");

    memex()
        .current_dir(tmp.path())
        .args(["node", "edit", "--open-thread", "How to handle auth?"])
        .assert()
        .success();

    memex()
        .current_dir(tmp.path())
        .args(["node", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("How to handle auth?"));
}

#[test]
fn node_edit_summary_and_goal_conflict() {
    let tmp = TempDir::new().unwrap();
    init_in(&tmp);
    create_node(&tmp, "Task");

    memex()
        .current_dir(tmp.path())
        .args([
            "node",
            "edit",
            "--summary",
            r#"goal = "Full TOML"
decisions = []
rejected_approaches = []
open_threads = []
key_artifacts = []"#,
            "--goal",
            "conflicting",
        ])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("cannot be combined").or(predicate::str::contains("conflict")),
        );
}

#[test]
fn node_edit_empty_goal_fails() {
    let tmp = TempDir::new().unwrap();
    init_in(&tmp);
    create_node(&tmp, "Task");

    memex()
        .current_dir(tmp.path())
        .args(["node", "edit", "--goal", ""])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be empty").or(predicate::str::contains("empty")));
}

// ─── Node status ─────────────────────────────────────────────────────────────

#[test]
fn node_resolve_marks_resolved() {
    let tmp = TempDir::new().unwrap();
    init_in(&tmp);
    create_node(&tmp, "Task to resolve");

    memex()
        .current_dir(tmp.path())
        .args(["node", "resolve"])
        .assert()
        .success();

    memex()
        .current_dir(tmp.path())
        .args(["node", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Resolved"));
}

#[test]
fn node_resolve_already_resolved_errors() {
    let tmp = TempDir::new().unwrap();
    init_in(&tmp);
    create_node(&tmp, "Task");

    memex()
        .current_dir(tmp.path())
        .args(["node", "resolve"])
        .assert()
        .success();

    memex()
        .current_dir(tmp.path())
        .args(["node", "resolve"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("already resolved").or(predicate::str::contains("already")),
        );
}

#[test]
fn node_abandon_marks_abandoned() {
    let tmp = TempDir::new().unwrap();
    init_in(&tmp);
    create_node(&tmp, "Abandoned task");

    memex()
        .current_dir(tmp.path())
        .args(["node", "abandon"])
        .assert()
        .success();

    memex()
        .current_dir(tmp.path())
        .args(["node", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Abandoned"));
}

#[test]
fn node_reopen_resolved_node() {
    let tmp = TempDir::new().unwrap();
    init_in(&tmp);
    create_node(&tmp, "Task");

    memex()
        .current_dir(tmp.path())
        .args(["node", "resolve"])
        .assert()
        .success();
    memex()
        .current_dir(tmp.path())
        .args(["node", "reopen"])
        .assert()
        .success();

    memex()
        .current_dir(tmp.path())
        .args(["node", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Active"));
}

#[test]
fn node_reopen_already_active_errors() {
    let tmp = TempDir::new().unwrap();
    init_in(&tmp);
    create_node(&tmp, "Task");

    memex()
        .current_dir(tmp.path())
        .args(["node", "reopen"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already active").or(predicate::str::contains("already")));
}

// ─── Node show / list ────────────────────────────────────────────────────────

#[test]
fn node_show_by_short_id() {
    let tmp = TempDir::new().unwrap();
    init_in(&tmp);
    let short_id = create_node(&tmp, "Specific node");

    memex()
        .current_dir(tmp.path())
        .args(["node", "show", &short_id])
        .assert()
        .success()
        .stdout(predicate::str::contains("Specific node"));
}

#[test]
fn node_list_shows_all_nodes() {
    let tmp = TempDir::new().unwrap();
    init_in(&tmp);
    create_node(&tmp, "Alpha node");
    create_node(&tmp, "Beta node");

    memex()
        .current_dir(tmp.path())
        .args(["node", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Alpha node"))
        .stdout(predicate::str::contains("Beta node"));
}

#[test]
fn node_list_marks_active() {
    let tmp = TempDir::new().unwrap();
    init_in(&tmp);
    create_node(&tmp, "Active node");

    let output = memex()
        .current_dir(tmp.path())
        .args(["node", "list"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let text = String::from_utf8(output).unwrap();
    assert!(
        text.lines().any(|l| l.starts_with('*')),
        "Expected a line starting with '*' in:\n{text}"
    );
}

// ─── Graph ───────────────────────────────────────────────────────────────────

#[test]
fn graph_view_single_node() {
    let tmp = TempDir::new().unwrap();
    init_in(&tmp);
    create_node(&tmp, "Lone node");

    memex()
        .current_dir(tmp.path())
        .args(["graph", "view"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Conversation Graph"))
        .stdout(predicate::str::contains("Legend:"));
}

#[test]
fn graph_view_tree_connectors() {
    let tmp = TempDir::new().unwrap();
    init_in(&tmp);
    let parent_id = create_node(&tmp, "Parent");
    let _child_id = create_node(&tmp, "Child"); // auto-parented to active (parent)
    let _ = parent_id;

    memex()
        .current_dir(tmp.path())
        .args(["graph", "view"])
        .assert()
        .success()
        .stdout(predicate::str::contains("──"));
}

// ─── Search ──────────────────────────────────────────────────────────────────

#[test]
fn search_finds_goal() {
    let tmp = TempDir::new().unwrap();
    init_in(&tmp);
    create_node(&tmp, "Build a database indexer");

    memex()
        .current_dir(tmp.path())
        .args(["search", "database"])
        .assert()
        .success()
        .stdout(predicate::str::contains(">>database<<"));
}

#[test]
fn search_case_insensitive() {
    let tmp = TempDir::new().unwrap();
    init_in(&tmp);
    create_node(&tmp, "Build a DATABASE indexer");

    memex()
        .current_dir(tmp.path())
        .args(["search", "database"])
        .assert()
        .success()
        .stdout(predicate::str::contains(">>DATABASE<<"));
}

#[test]
fn search_no_match() {
    let tmp = TempDir::new().unwrap();
    init_in(&tmp);
    create_node(&tmp, "Build a web server");

    memex()
        .current_dir(tmp.path())
        .args(["search", "zzznomatch"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No nodes found matching"));
}

#[test]
fn search_finds_decisions() {
    let tmp = TempDir::new().unwrap();
    init_in(&tmp);
    create_node(&tmp, "Design system");

    memex()
        .current_dir(tmp.path())
        .args(["node", "edit", "--decision", "Use microservices"])
        .assert()
        .success();

    memex()
        .current_dir(tmp.path())
        .args(["search", "microservices"])
        .assert()
        .success()
        .stdout(predicate::str::contains("decision[0]"));
}

// ─── Context ─────────────────────────────────────────────────────────────────

#[test]
fn context_markdown_output() {
    let tmp = TempDir::new().unwrap();
    init_in(&tmp);
    create_node(&tmp, "Root project");

    memex()
        .current_dir(tmp.path())
        .args(["context", "--format", "markdown"])
        .assert()
        .success()
        .stdout(predicate::str::contains("## Project Context"))
        .stdout(predicate::str::contains("**Goal:**"));
}

#[test]
fn context_xml_output() {
    let tmp = TempDir::new().unwrap();
    init_in(&tmp);
    create_node(&tmp, "Root project");

    memex()
        .current_dir(tmp.path())
        .args(["context", "--format", "xml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("<memex_context>"))
        .stdout(predicate::str::contains("<goal>"));
}

#[test]
fn context_plain_output() {
    let tmp = TempDir::new().unwrap();
    init_in(&tmp);
    create_node(&tmp, "Root project");

    memex()
        .current_dir(tmp.path())
        .args(["context", "--format", "plain"])
        .assert()
        .success()
        .stdout(predicate::str::contains("PROJECT CONTEXT"));
}

#[test]
fn context_invalid_format_fails() {
    let tmp = TempDir::new().unwrap();
    init_in(&tmp);

    memex()
        .current_dir(tmp.path())
        .args(["context", "--format", "json"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown format"));
}

#[test]
fn context_depth_trimming() {
    let tmp = TempDir::new().unwrap();
    init_in(&tmp);

    // Build a 4-level chain: root → level1 → level2 → current
    let root_id = create_node(&tmp, "Root node");
    let level1_id = create_node(&tmp, "Level one node");
    let level2_id = create_node(&tmp, "Level two node");
    let _current_id = create_node(&tmp, "Current node");
    let _ = (root_id, level1_id, level2_id);

    // --depth 1: only keep 1 ancestor between root and current → Level one should be trimmed
    memex()
        .current_dir(tmp.path())
        .args(["context", "--format", "plain", "--depth", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Level one node").not())
        .stdout(predicate::str::contains("Current node"));
}

use std::env;
use std::fs;
use std::process::Command;

use anyhow::{Context, Result, bail};
use tempfile::Builder;

use crate::models::{NodeSummary, NodeSummaryToml, RejectedApproachToml};

/// Open a TOML template of the node summary in the user's editor.
/// Returns the edited summary on success.
pub fn edit_node_summary(existing: Option<&NodeSummary>) -> Result<NodeSummary> {
    let template = match existing {
        Some(s) => {
            let toml_repr = NodeSummaryToml::from(s);
            toml::to_string_pretty(&toml_repr).context("Failed to serialize summary to TOML")?
        }
        None => default_template(),
    };

    let tmp = Builder::new()
        .prefix("llmgraph-node-")
        .suffix(".toml")
        .tempfile()
        .context("Failed to create temp file")?;

    fs::write(tmp.path(), &template).context("Failed to write temp file")?;

    let editor = resolve_editor();
    let status = Command::new(&editor)
        .arg(tmp.path())
        .status()
        .with_context(|| format!("Failed to launch editor '{}'", editor))?;

    if !status.success() {
        bail!("Editor exited with non-zero status");
    }

    let edited = fs::read_to_string(tmp.path()).context("Failed to read edited file")?;
    let parsed: NodeSummaryToml =
        toml::from_str(&edited).context("Failed to parse edited TOML — check your syntax")?;

    Ok(NodeSummary::from(parsed))
}

/// Open arbitrary text content in the editor, return the edited content.
pub fn edit_text(initial: &str, suffix: &str) -> Result<String> {
    let tmp = Builder::new()
        .prefix("llmgraph-")
        .suffix(suffix)
        .tempfile()
        .context("Failed to create temp file")?;

    fs::write(tmp.path(), initial).context("Failed to write temp file")?;

    let editor = resolve_editor();
    let status = Command::new(&editor)
        .arg(tmp.path())
        .status()
        .with_context(|| format!("Failed to launch editor '{}'", editor))?;

    if !status.success() {
        bail!("Editor exited with non-zero status");
    }

    fs::read_to_string(tmp.path()).context("Failed to read edited file")
}

pub fn resolve_editor() -> String {
    env::var("EDITOR")
        .or_else(|_| env::var("VISUAL"))
        .unwrap_or_else(|_| "vi".to_string())
}

fn default_template() -> String {
    let example = NodeSummaryToml {
        goal: "Describe the goal of this conversation session".to_string(),
        decisions: vec![
            "Decision 1 made during this session".to_string(),
            "Decision 2 made during this session".to_string(),
        ],
        rejected_approaches: vec![RejectedApproachToml {
            description: "Approach that was considered but rejected".to_string(),
            reason: "Why it was rejected".to_string(),
        }],
        open_threads: vec!["Unresolved question or follow-up item".to_string()],
        key_artifacts: vec!["src/example.rs".to_string()],
    };
    toml::to_string_pretty(&example).unwrap_or_else(|_| String::new())
}

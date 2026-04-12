use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum NodeStatus {
    Active,
    Resolved,
    Abandoned,
}

impl std::fmt::Display for NodeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeStatus::Active => write!(f, "Active"),
            NodeStatus::Resolved => write!(f, "Resolved"),
            NodeStatus::Abandoned => write!(f, "Abandoned"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RejectedApproach {
    pub description: String,
    pub reason: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct NodeSummary {
    pub goal: String,
    pub decisions: Vec<String>,
    pub rejected_approaches: Vec<RejectedApproach>,
    pub open_threads: Vec<String>,
    pub key_artifacts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationNode {
    pub id: Uuid,
    pub parent_ids: Vec<Uuid>,
    pub git_ref: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub summary: NodeSummary,
    pub raw_transcript_ref: Option<String>,
    pub tags: Vec<String>,
    pub status: NodeStatus,
}

impl ConversationNode {
    pub fn new(parent_ids: Vec<Uuid>, git_ref: Option<String>, tags: Vec<String>) -> Self {
        let now = Utc::now();
        ConversationNode {
            id: Uuid::new_v4(),
            parent_ids,
            git_ref,
            created_at: now,
            updated_at: now,
            summary: NodeSummary::default(),
            raw_transcript_ref: None,
            tags,
            status: NodeStatus::Active,
        }
    }

    pub fn short_id(&self) -> String {
        self.id.to_string()[..8].to_string()
    }

    pub fn status_icon(&self) -> &str {
        match self.status {
            NodeStatus::Active => "●",
            NodeStatus::Resolved => "✓",
            NodeStatus::Abandoned => "✗",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub from: Uuid,
    pub to: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Graph {
    pub version: String,
    pub root_id: Option<Uuid>,
    pub edges: Vec<Edge>,
}

impl Graph {
    pub fn new() -> Self {
        Graph {
            version: "1".to_string(),
            root_id: None,
            edges: Vec::new(),
        }
    }

    pub fn add_edge(&mut self, from: Uuid, to: Uuid) {
        self.edges.push(Edge { from, to });
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub active_id: Option<Uuid>,
}

impl State {
    pub fn new() -> Self {
        State { active_id: None }
    }
}

/// TOML-friendly representation of a node summary for editing
#[derive(Debug, Serialize, Deserialize)]
pub struct NodeSummaryToml {
    pub goal: String,
    pub decisions: Vec<String>,
    pub rejected_approaches: Vec<RejectedApproachToml>,
    pub open_threads: Vec<String>,
    pub key_artifacts: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RejectedApproachToml {
    pub description: String,
    pub reason: String,
}

impl From<&NodeSummary> for NodeSummaryToml {
    fn from(s: &NodeSummary) -> Self {
        NodeSummaryToml {
            goal: s.goal.clone(),
            decisions: s.decisions.clone(),
            rejected_approaches: s
                .rejected_approaches
                .iter()
                .map(|r| RejectedApproachToml {
                    description: r.description.clone(),
                    reason: r.reason.clone(),
                })
                .collect(),
            open_threads: s.open_threads.clone(),
            key_artifacts: s.key_artifacts.clone(),
        }
    }
}

impl From<NodeSummaryToml> for NodeSummary {
    fn from(t: NodeSummaryToml) -> Self {
        NodeSummary {
            goal: t.goal,
            decisions: t.decisions,
            rejected_approaches: t
                .rejected_approaches
                .into_iter()
                .map(|r| RejectedApproach {
                    description: r.description,
                    reason: r.reason,
                })
                .collect(),
            open_threads: t.open_threads,
            key_artifacts: t.key_artifacts,
        }
    }
}

/// Config file structure
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub llm: LlmConfig,
    #[serde(default)]
    pub git: GitConfig,
    #[serde(default)]
    pub storage: StorageConfig,
    #[serde(default)]
    pub ui: UiConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LlmConfig {
    pub provider: String,
    pub api_key_env: String,
    pub model: String,
    pub base_url: String,
}

impl Default for LlmConfig {
    fn default() -> Self {
        LlmConfig {
            provider: "anthropic".to_string(),
            api_key_env: "ANTHROPIC_API_KEY".to_string(),
            model: "claude-sonnet-4-20250514".to_string(),
            base_url: String::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitConfig {
    pub auto_prompt_on_branch: bool,
    pub annotate_on_commit: bool,
}

impl Default for GitConfig {
    fn default() -> Self {
        GitConfig {
            auto_prompt_on_branch: true,
            annotate_on_commit: false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StorageConfig {
    pub track_in_git: bool,
    pub transcript_storage: String,
}

impl Default for StorageConfig {
    fn default() -> Self {
        StorageConfig {
            track_in_git: true,
            transcript_storage: "none".to_string(),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct UiConfig {
    pub editor: String,
}

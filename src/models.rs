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
    pub git: GitConfig,
    #[serde(default)]
    pub storage: StorageConfig,
    #[serde(default)]
    pub ui: UiConfig,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_status_display() {
        assert_eq!(format!("{}", NodeStatus::Active), "Active");
        assert_eq!(format!("{}", NodeStatus::Resolved), "Resolved");
        assert_eq!(format!("{}", NodeStatus::Abandoned), "Abandoned");
    }

    #[test]
    fn node_status_icons() {
        let mut node = ConversationNode::new(vec![], None, vec![]);
        node.status = NodeStatus::Active;
        assert_eq!(node.status_icon(), "●");
        node.status = NodeStatus::Resolved;
        assert_eq!(node.status_icon(), "✓");
        node.status = NodeStatus::Abandoned;
        assert_eq!(node.status_icon(), "✗");
    }

    #[test]
    fn node_status_serde_roundtrip() {
        let json = serde_json::to_string(&NodeStatus::Resolved).unwrap();
        assert_eq!(json, "\"Resolved\"");
        let back: NodeStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(back, NodeStatus::Resolved);

        let json2 = serde_json::to_string(&NodeStatus::Abandoned).unwrap();
        assert_eq!(json2, "\"Abandoned\"");
        let back2: NodeStatus = serde_json::from_str(&json2).unwrap();
        assert_eq!(back2, NodeStatus::Abandoned);
    }

    #[test]
    fn short_id_is_8_chars() {
        let node = ConversationNode::new(vec![], None, vec![]);
        let short = node.short_id();
        assert_eq!(short.len(), 8);
        assert!(node.id.to_string().starts_with(&short));
    }

    #[test]
    fn node_summary_default_is_empty() {
        let s = NodeSummary::default();
        assert!(s.goal.is_empty());
        assert!(s.decisions.is_empty());
        assert!(s.rejected_approaches.is_empty());
        assert!(s.open_threads.is_empty());
        assert!(s.key_artifacts.is_empty());
    }

    #[test]
    fn node_summary_toml_roundtrip() {
        let original = NodeSummary {
            goal: "Build a parser".to_string(),
            decisions: vec!["Use pest".to_string(), "Hand-roll lexer".to_string()],
            rejected_approaches: vec![RejectedApproach {
                description: "nom combinator".to_string(),
                reason: "Too verbose".to_string(),
            }],
            open_threads: vec!["Error recovery strategy?".to_string()],
            key_artifacts: vec!["src/parser.rs".to_string()],
        };

        let toml_repr = NodeSummaryToml::from(&original);
        let roundtripped = NodeSummary::from(toml_repr);

        assert_eq!(roundtripped.goal, original.goal);
        assert_eq!(roundtripped.decisions, original.decisions);
        assert_eq!(roundtripped.open_threads, original.open_threads);
        assert_eq!(roundtripped.key_artifacts, original.key_artifacts);
        assert_eq!(roundtripped.rejected_approaches.len(), 1);
        assert_eq!(
            roundtripped.rejected_approaches[0].description,
            original.rejected_approaches[0].description
        );
        assert_eq!(
            roundtripped.rejected_approaches[0].reason,
            original.rejected_approaches[0].reason
        );
    }

    #[test]
    fn node_summary_toml_roundtrip_empty() {
        let original = NodeSummary::default();
        let roundtripped = NodeSummary::from(NodeSummaryToml::from(&original));
        assert!(roundtripped.goal.is_empty());
        assert!(roundtripped.decisions.is_empty());
        assert!(roundtripped.rejected_approaches.is_empty());
    }

    #[test]
    fn node_json_roundtrip() {
        let mut node = ConversationNode::new(
            vec![Uuid::new_v4()],
            Some("main (abc12345)".to_string()),
            vec!["feat".to_string()],
        );
        node.summary.goal = "Ship the feature".to_string();
        node.summary.decisions.push("Use async".to_string());
        node.summary.rejected_approaches.push(RejectedApproach {
            description: "Threads".to_string(),
            reason: "Complexity".to_string(),
        });
        node.status = NodeStatus::Resolved;

        let json = serde_json::to_string(&node).unwrap();
        let back: ConversationNode = serde_json::from_str(&json).unwrap();

        assert_eq!(back.id, node.id);
        assert_eq!(back.parent_ids, node.parent_ids);
        assert_eq!(back.git_ref, node.git_ref);
        assert_eq!(back.tags, node.tags);
        assert_eq!(back.status, NodeStatus::Resolved);
        assert_eq!(back.summary.goal, "Ship the feature");
        assert_eq!(back.summary.decisions, vec!["Use async"]);
        assert_eq!(back.summary.rejected_approaches.len(), 1);
        assert_eq!(back.summary.rejected_approaches[0].description, "Threads");
    }

    #[test]
    fn state_json_roundtrip() {
        let id = Uuid::new_v4();
        let s = State {
            active_id: Some(id),
        };
        let json = serde_json::to_string(&s).unwrap();
        let back: State = serde_json::from_str(&json).unwrap();
        assert_eq!(back.active_id, Some(id));

        let empty = State::new();
        let json2 = serde_json::to_string(&empty).unwrap();
        let back2: State = serde_json::from_str(&json2).unwrap();
        assert!(back2.active_id.is_none());
    }
}

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use uuid::Uuid;

use crate::models::{Config, ConversationNode, State};

pub struct GraphStore {
    pub root: PathBuf,
}

impl GraphStore {
    /// Find the nearest `.memex/` directory by walking up from `cwd`.
    pub fn find(cwd: &Path) -> Option<PathBuf> {
        let mut dir = cwd.to_path_buf();
        loop {
            let candidate = dir.join(".memex");
            if candidate.is_dir() {
                return Some(dir);
            }
            if !dir.pop() {
                return None;
            }
        }
    }

    /// Open a store rooted at `root` (the project root, not the .memex dir).
    pub fn open(root: PathBuf) -> Self {
        GraphStore { root }
    }

    /// Open by searching from cwd, or return an error if not initialized.
    pub fn open_from_cwd() -> Result<Self> {
        let cwd = std::env::current_dir().context("Failed to get current directory")?;
        match Self::find(&cwd) {
            Some(root) => Ok(GraphStore::open(root)),
            None => bail!("No .memex directory found. Run `memex init` to initialize."),
        }
    }

    pub fn memex_dir(&self) -> PathBuf {
        self.root.join(".memex")
    }

    pub fn nodes_dir(&self) -> PathBuf {
        self.memex_dir().join("nodes")
    }

    pub fn state_path(&self) -> PathBuf {
        self.memex_dir().join("state.json")
    }

    pub fn config_path(&self) -> PathBuf {
        self.memex_dir().join("config.toml")
    }

    // -------------------------------------------------------------------------
    // Initialization
    // -------------------------------------------------------------------------

    pub fn is_initialized(&self) -> bool {
        self.memex_dir().is_dir()
    }

    pub fn initialize(&self) -> Result<()> {
        let dir = self.memex_dir();
        fs::create_dir_all(&dir).context("Failed to create .memex directory")?;
        fs::create_dir_all(self.nodes_dir()).context("Failed to create nodes directory")?;

        // Write default config if not present
        if !self.config_path().exists() {
            let config = Config::default();
            let toml_str = toml::to_string_pretty(&config).context("Failed to serialize config")?;
            fs::write(self.config_path(), toml_str).context("Failed to write config.toml")?;
        }

        // Write .gitignore to keep state.json out of git; never overwrite a user-customized file
        let gitignore_path = dir.join(".gitignore");
        if !gitignore_path.exists() {
            fs::write(&gitignore_path, "state.json\n")
                .context("Failed to write .memex/.gitignore")?;
        }

        Ok(())
    }

    // -------------------------------------------------------------------------
    // State
    // -------------------------------------------------------------------------

    pub fn load_state(&self) -> Result<State> {
        let path = self.state_path();
        if !path.exists() {
            return Ok(State::new());
        }
        let data = fs::read_to_string(&path).context("Failed to read state.json")?;
        serde_json::from_str(&data).context("Failed to parse state.json")
    }

    pub fn save_state(&self, state: &State) -> Result<()> {
        let data = serde_json::to_string_pretty(state).context("Failed to serialize state")?;
        fs::write(self.state_path(), data).context("Failed to write state.json")
    }

    pub fn get_active_id(&self) -> Result<Option<Uuid>> {
        let state = self.load_state()?;
        Ok(state.active_id)
    }

    pub fn set_active_id(&self, id: Uuid) -> Result<()> {
        let mut state = self.load_state()?;
        state.active_id = Some(id);
        self.save_state(&state)
    }

    // -------------------------------------------------------------------------
    // Nodes
    // -------------------------------------------------------------------------

    pub fn node_path(&self, id: Uuid) -> PathBuf {
        self.nodes_dir().join(format!("{}.json", id))
    }

    pub fn save_node(&self, node: &ConversationNode) -> Result<()> {
        let data = serde_json::to_string_pretty(node).context("Failed to serialize node")?;
        fs::write(self.node_path(node.id), data).context("Failed to write node file")
    }

    pub fn load_node(&self, id: Uuid) -> Result<ConversationNode> {
        let path = self.node_path(id);
        let data =
            fs::read_to_string(&path).with_context(|| format!("Failed to read node {}", id))?;
        serde_json::from_str(&data).with_context(|| format!("Failed to parse node {}", id))
    }

    pub fn load_all_nodes(&self) -> Result<Vec<ConversationNode>> {
        let dir = self.nodes_dir();
        let mut nodes = Vec::new();
        if !dir.exists() {
            return Ok(nodes);
        }
        for entry in fs::read_dir(&dir).context("Failed to read nodes directory")? {
            let entry = entry.context("Failed to read directory entry")?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                let data = fs::read_to_string(&path)
                    .with_context(|| format!("Failed to read {}", path.display()))?;
                let node: ConversationNode = serde_json::from_str(&data)
                    .with_context(|| format!("Failed to parse {}", path.display()))?;
                nodes.push(node);
            }
        }
        // Sort by created_at for consistent output
        nodes.sort_by_key(|n| n.created_at);
        Ok(nodes)
    }

    /// Resolve a node ID from an optional short or full ID string, falling back to active node.
    pub fn resolve_node_id(&self, id_opt: Option<&str>) -> Result<Uuid> {
        match id_opt {
            Some(id_str) => self.find_node_id_by_prefix(id_str),
            None => {
                let state = self.load_state()?;
                state
                    .active_id
                    .ok_or_else(|| anyhow::anyhow!("No active node. Specify a node ID."))
            }
        }
    }

    /// Find a node ID that starts with the given prefix (supports short IDs).
    pub fn find_node_id_by_prefix(&self, prefix: &str) -> Result<Uuid> {
        // Try exact UUID parse first
        if let Ok(id) = uuid::Uuid::parse_str(prefix) {
            return Ok(id);
        }

        // Search by prefix
        let nodes = self.load_all_nodes()?;
        let matches: Vec<&ConversationNode> = nodes
            .iter()
            .filter(|n| n.id.to_string().starts_with(prefix))
            .collect();

        match matches.len() {
            0 => bail!("No node found with ID prefix '{}'", prefix),
            1 => Ok(matches[0].id),
            _ => bail!(
                "Ambiguous ID prefix '{}' matches {} nodes",
                prefix,
                matches.len()
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone, Utc};
    use tempfile::TempDir;

    fn make_store() -> (TempDir, GraphStore) {
        let tmp = TempDir::new().unwrap();
        let store = GraphStore::open(tmp.path().to_path_buf());
        store.initialize().unwrap();
        (tmp, store)
    }

    // --- Discovery ---

    #[test]
    fn find_returns_none_without_memex() {
        let tmp = TempDir::new().unwrap();
        assert!(GraphStore::find(tmp.path()).is_none());
    }

    #[test]
    fn find_returns_root_when_memex_present() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join(".memex")).unwrap();
        let found = GraphStore::find(tmp.path()).unwrap();
        assert_eq!(found, tmp.path());
    }

    #[test]
    fn find_walks_up_tree() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join(".memex")).unwrap();
        let nested = tmp.path().join("a").join("b").join("c");
        fs::create_dir_all(&nested).unwrap();
        let found = GraphStore::find(&nested).unwrap();
        assert_eq!(found, tmp.path());
    }

    // --- Initialization ---

    #[test]
    fn is_initialized_false_before_init() {
        let tmp = TempDir::new().unwrap();
        let store = GraphStore::open(tmp.path().to_path_buf());
        assert!(!store.is_initialized());
    }

    #[test]
    fn is_initialized_true_after_init() {
        let (_tmp, store) = make_store();
        assert!(store.is_initialized());
    }

    #[test]
    fn initialize_creates_structure() {
        let tmp = TempDir::new().unwrap();
        let store = GraphStore::open(tmp.path().to_path_buf());
        store.initialize().unwrap();
        assert!(store.memex_dir().is_dir());
        assert!(store.nodes_dir().is_dir());
        assert!(store.config_path().exists());
        let config_content = fs::read_to_string(store.config_path()).unwrap();
        assert!(!config_content.is_empty());
        let gitignore = fs::read_to_string(store.memex_dir().join(".gitignore")).unwrap();
        assert!(gitignore.contains("state.json"));
    }

    #[test]
    fn initialize_does_not_overwrite_config() {
        let (_tmp, store) = make_store();
        fs::write(store.config_path(), "custom = true").unwrap();
        store.initialize().unwrap();
        let content = fs::read_to_string(store.config_path()).unwrap();
        assert_eq!(content, "custom = true");
    }

    // --- Node I/O ---

    #[test]
    fn save_and_load_node_roundtrip() {
        let (_tmp, store) = make_store();
        let mut node = ConversationNode::new(vec![], None, vec![]);
        node.summary.goal = "Test goal".to_string();
        node.summary.decisions.push("Decision A".to_string());

        store.save_node(&node).unwrap();
        let loaded = store.load_node(node.id).unwrap();

        assert_eq!(loaded.id, node.id);
        assert_eq!(loaded.summary.goal, "Test goal");
        assert_eq!(loaded.summary.decisions, vec!["Decision A"]);
    }

    #[test]
    fn load_node_error_on_missing() {
        let (_tmp, store) = make_store();
        let result = store.load_node(Uuid::new_v4());
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("Failed to read node"),
            "unexpected error: {}",
            msg
        );
    }

    #[test]
    fn load_all_nodes_empty_without_nodes() {
        let (_tmp, store) = make_store();
        let nodes = store.load_all_nodes().unwrap();
        assert!(nodes.is_empty());
    }

    #[test]
    fn load_all_nodes_sorted_by_created_at() {
        let (_tmp, store) = make_store();
        let base = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();

        let mut n1 = ConversationNode::new(vec![], None, vec![]);
        n1.created_at = base;
        n1.summary.goal = "First".to_string();

        let mut n2 = ConversationNode::new(vec![], None, vec![]);
        n2.created_at = base + Duration::seconds(10);
        n2.summary.goal = "Second".to_string();

        let mut n3 = ConversationNode::new(vec![], None, vec![]);
        n3.created_at = base + Duration::seconds(20);
        n3.summary.goal = "Third".to_string();

        // Save in reverse order to ensure sorting is actually applied
        store.save_node(&n3).unwrap();
        store.save_node(&n1).unwrap();
        store.save_node(&n2).unwrap();

        let loaded = store.load_all_nodes().unwrap();
        assert_eq!(loaded.len(), 3);
        assert_eq!(loaded[0].summary.goal, "First");
        assert_eq!(loaded[1].summary.goal, "Second");
        assert_eq!(loaded[2].summary.goal, "Third");
    }

    #[test]
    fn load_all_nodes_ignores_non_json_files() {
        let (_tmp, store) = make_store();
        let node = ConversationNode::new(vec![], None, vec![]);
        store.save_node(&node).unwrap();
        fs::write(store.nodes_dir().join("readme.txt"), "ignore me").unwrap();

        let nodes = store.load_all_nodes().unwrap();
        assert_eq!(nodes.len(), 1);
    }

    // --- State I/O ---

    #[test]
    fn save_and_load_state_roundtrip() {
        let (_tmp, store) = make_store();
        let id = Uuid::new_v4();
        store.set_active_id(id).unwrap();
        let active = store.get_active_id().unwrap();
        assert_eq!(active, Some(id));
    }

    #[test]
    fn load_state_returns_none_when_missing() {
        let (_tmp, store) = make_store();
        let state = store.load_state().unwrap();
        assert!(state.active_id.is_none());
    }

    // --- ID resolution ---

    #[test]
    fn resolve_node_id_uses_active_when_none() {
        let (_tmp, store) = make_store();
        let node = ConversationNode::new(vec![], None, vec![]);
        store.save_node(&node).unwrap();
        store.set_active_id(node.id).unwrap();

        let resolved = store.resolve_node_id(None).unwrap();
        assert_eq!(resolved, node.id);
    }

    #[test]
    fn resolve_node_id_errors_without_active() {
        let (_tmp, store) = make_store();
        let result = store.resolve_node_id(None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No active node"));
    }

    #[test]
    fn resolve_node_id_uses_prefix() {
        let (_tmp, store) = make_store();
        let node = ConversationNode::new(vec![], None, vec![]);
        store.save_node(&node).unwrap();

        let resolved = store.resolve_node_id(Some(&node.short_id())).unwrap();
        assert_eq!(resolved, node.id);
    }

    #[test]
    fn find_node_by_prefix_exact_uuid() {
        let (_tmp, store) = make_store();
        let node = ConversationNode::new(vec![], None, vec![]);
        store.save_node(&node).unwrap();

        let found = store.find_node_id_by_prefix(&node.id.to_string()).unwrap();
        assert_eq!(found, node.id);
    }

    #[test]
    fn find_node_by_prefix_short() {
        let (_tmp, store) = make_store();
        let node = ConversationNode::new(vec![], None, vec![]);
        store.save_node(&node).unwrap();

        let prefix = &node.id.to_string()[..6];
        let found = store.find_node_id_by_prefix(prefix).unwrap();
        assert_eq!(found, node.id);
    }

    #[test]
    fn find_node_by_prefix_ambiguous() {
        let (_tmp, store) = make_store();
        let mut n1 = ConversationNode::new(vec![], None, vec![]);
        n1.id = Uuid::parse_str("aaaaaaaa-1111-1111-1111-111111111111").unwrap();
        let mut n2 = ConversationNode::new(vec![], None, vec![]);
        n2.id = Uuid::parse_str("aaaaaaaa-2222-2222-2222-222222222222").unwrap();

        store.save_node(&n1).unwrap();
        store.save_node(&n2).unwrap();

        let result = store.find_node_id_by_prefix("aaaaaaaa");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("Ambiguous"), "unexpected error: {}", msg);
    }

    #[test]
    fn find_node_by_prefix_not_found() {
        let (_tmp, store) = make_store();
        let result = store.find_node_id_by_prefix("00000000");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("No node found"), "unexpected error: {}", msg);
    }
}

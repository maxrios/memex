use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use uuid::Uuid;

use crate::models::{Config, ConversationNode, Graph, State};

pub struct GraphStore {
    pub root: PathBuf,
}

impl GraphStore {
    /// Find the nearest `.llmgraph/` directory by walking up from `cwd`.
    pub fn find(cwd: &Path) -> Option<PathBuf> {
        let mut dir = cwd.to_path_buf();
        loop {
            let candidate = dir.join(".llmgraph");
            if candidate.is_dir() {
                return Some(dir);
            }
            if !dir.pop() {
                return None;
            }
        }
    }

    /// Open a store rooted at `root` (the project root, not the .llmgraph dir).
    pub fn open(root: PathBuf) -> Self {
        GraphStore { root }
    }

    /// Open by searching from cwd, or return an error if not initialized.
    pub fn open_from_cwd() -> Result<Self> {
        let cwd = std::env::current_dir().context("Failed to get current directory")?;
        match Self::find(&cwd) {
            Some(root) => Ok(GraphStore::open(root)),
            None => bail!(
                "No .llmgraph directory found. Run `llmgraph init` to initialize."
            ),
        }
    }

    pub fn llmgraph_dir(&self) -> PathBuf {
        self.root.join(".llmgraph")
    }

    pub fn nodes_dir(&self) -> PathBuf {
        self.llmgraph_dir().join("nodes")
    }

    pub fn graph_path(&self) -> PathBuf {
        self.llmgraph_dir().join("graph.json")
    }

    pub fn state_path(&self) -> PathBuf {
        self.llmgraph_dir().join("state.json")
    }

    pub fn config_path(&self) -> PathBuf {
        self.llmgraph_dir().join("config.toml")
    }

    // -------------------------------------------------------------------------
    // Initialization
    // -------------------------------------------------------------------------

    pub fn is_initialized(&self) -> bool {
        self.llmgraph_dir().is_dir()
    }

    pub fn initialize(&self) -> Result<()> {
        let dir = self.llmgraph_dir();
        fs::create_dir_all(&dir).context("Failed to create .llmgraph directory")?;
        fs::create_dir_all(self.nodes_dir()).context("Failed to create nodes directory")?;

        // Write default config if not present
        if !self.config_path().exists() {
            let config = Config::default();
            let toml_str =
                toml::to_string_pretty(&config).context("Failed to serialize config")?;
            fs::write(self.config_path(), toml_str).context("Failed to write config.toml")?;
        }

        Ok(())
    }

    // -------------------------------------------------------------------------
    // Graph
    // -------------------------------------------------------------------------

    pub fn load_graph(&self) -> Result<Graph> {
        let path = self.graph_path();
        if !path.exists() {
            return Ok(Graph::new());
        }
        let data = fs::read_to_string(&path).context("Failed to read graph.json")?;
        serde_json::from_str(&data).context("Failed to parse graph.json")
    }

    pub fn save_graph(&self, graph: &Graph) -> Result<()> {
        let data = serde_json::to_string_pretty(graph).context("Failed to serialize graph")?;
        fs::write(self.graph_path(), data).context("Failed to write graph.json")
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
        let data = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read node {}", id))?;
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
                let data =
                    fs::read_to_string(&path).with_context(|| {
                        format!("Failed to read {}", path.display())
                    })?;
                let node: ConversationNode = serde_json::from_str(&data)
                    .with_context(|| format!("Failed to parse {}", path.display()))?;
                nodes.push(node);
            }
        }
        // Sort by created_at for consistent output
        nodes.sort_by(|a, b| a.created_at.cmp(&b.created_at));
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

    // -------------------------------------------------------------------------
    // Config
    // -------------------------------------------------------------------------

    #[allow(dead_code)]
    pub fn load_config(&self) -> Result<Config> {
        let path = self.config_path();
        if !path.exists() {
            return Ok(Config::default());
        }
        let data = fs::read_to_string(&path).context("Failed to read config.toml")?;
        toml::from_str(&data).context("Failed to parse config.toml")
    }
}

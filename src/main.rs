mod commands;
mod editor;
mod git;
mod models;
mod store;

use anyhow::Result;
use clap::{Parser, Subcommand};

use commands::context::OutputFormat;
use models::NodeStatus;

#[derive(Parser)]
#[command(
    name = "llmgraph",
    about = "Organize LLM conversations into a versioned, navigable DAG tied to a software project",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new llmgraph in the current directory
    Init,

    /// Manage conversation nodes
    Node {
        #[command(subcommand)]
        subcommand: NodeCommands,
    },

    /// View the conversation graph
    Graph {
        #[command(subcommand)]
        subcommand: GraphCommands,
    },

    /// Generate a context payload for LLM injection
    Context {
        /// Node ID (defaults to active node)
        id: Option<String>,

        /// Output format: markdown, xml, or plain
        #[arg(long, default_value = "markdown")]
        format: String,
    },

    /// Search node summaries
    Search {
        /// Query string to search for
        query: String,
    },
}

#[derive(Subcommand)]
enum NodeCommands {
    /// Create a new conversation node
    Create {
        /// Parent node ID (defaults to active node)
        #[arg(long)]
        parent: Option<String>,

        /// Git ref (branch/SHA) to associate with this node
        #[arg(long, name = "git-ref")]
        git_ref: Option<String>,

        /// Tags to attach to this node
        #[arg(long, name = "tag", num_args = 1)]
        tags: Vec<String>,
    },

    /// Edit a node's summary in $EDITOR
    Edit {
        /// Node ID (defaults to active node)
        id: Option<String>,
    },

    /// Show a node's full summary
    Show {
        /// Node ID (defaults to active node)
        id: Option<String>,
    },

    /// List all nodes
    List,

    /// Mark a node as resolved
    Resolve {
        /// Node ID (defaults to active node)
        id: Option<String>,
    },

    /// Mark a node as abandoned
    Abandon {
        /// Node ID (defaults to active node)
        id: Option<String>,
    },

    /// Reopen a resolved or abandoned node (set back to Active)
    Reopen {
        /// Node ID (defaults to active node)
        id: Option<String>,
    },
}

#[derive(Subcommand)]
enum GraphCommands {
    /// Render the graph as an ASCII tree
    View,
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("Error: {:#}", e);
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Init => commands::init::run(),

        Commands::Node { subcommand } => match subcommand {
            NodeCommands::Create { parent, git_ref, tags } => {
                commands::node::create(
                    parent.as_deref(),
                    git_ref.as_deref(),
                    &tags,
                )
            }
            NodeCommands::Edit { id } => commands::node::edit(id.as_deref()),
            NodeCommands::Show { id } => commands::node::show(id.as_deref()),
            NodeCommands::List => commands::node::list(),
            NodeCommands::Resolve { id } => {
                commands::node::set_status(id.as_deref(), NodeStatus::Resolved)
            }
            NodeCommands::Abandon { id } => {
                commands::node::set_status(id.as_deref(), NodeStatus::Abandoned)
            }
            NodeCommands::Reopen { id } => {
                commands::node::set_status(id.as_deref(), NodeStatus::Active)
            }
        },

        Commands::Graph { subcommand } => match subcommand {
            GraphCommands::View => commands::graph::view(),
        },

        Commands::Context { id, format } => {
            let fmt = OutputFormat::from_str(&format)?;
            commands::context::run(id.as_deref(), fmt)
        }

        Commands::Search { query } => commands::search::run(&query),
    }
}

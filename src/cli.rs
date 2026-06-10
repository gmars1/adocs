use camino::Utf8PathBuf;
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "adocs")]
#[command(about = "Local-first trust map for code repositories")]
pub struct Cli {
    #[arg(long, global = true)]
    pub source_root: Option<Utf8PathBuf>,

    #[arg(long, global = true)]
    pub map_root: Option<Utf8PathBuf>,

    #[arg(long, global = true)]
    pub config: Option<Utf8PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Init {
        #[arg(long)]
        force: bool,
    },
    Sync,
    Status {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        fail_on_stale: bool,
        #[arg(long)]
        fail_on_missing_docs: bool,
        #[arg(long)]
        fail_on_ambiguous: bool,
    },
    Changed {
        #[arg(long)]
        json: bool,
    },
    List {
        #[arg(long)]
        state: StateFilter,
        #[arg(long, default_value_t = KindFilter::All)]
        kind: KindFilter,
        #[arg(long)]
        json: bool,
    },
    Stale {
        #[arg(long)]
        json: bool,
    },
    Valid {
        #[arg(long)]
        json: bool,
    },
    Context {
        path: Utf8PathBuf,
    },
    Update {
        path: Utf8PathBuf,
    },
    Seal {
        path: Utf8PathBuf,
    },
    Rebind {
        file_id: String,
        new_path: Utf8PathBuf,
    },
    Serve {
        #[arg(long)]
        mcp: bool,
    },
    #[command(name = "docsunder")]
    DocsUnder {
        path: Utf8PathBuf,
        #[arg(long)]
        foldersonly: bool,
        #[arg(long)]
        filesonly: bool,
        #[arg(long)]
        json: bool,
    },
    InstallAgent {
        agent: String,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum StateFilter {
    Stale,
    Valid,
    Sealed,
    All,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum KindFilter {
    Files,
    Folders,
    All,
}

impl std::fmt::Display for KindFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KindFilter::Files => write!(f, "files"),
            KindFilter::Folders => write!(f, "folders"),
            KindFilter::All => write!(f, "all"),
        }
    }
}

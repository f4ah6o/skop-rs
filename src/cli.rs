use clap::{Parser, Subcommand, ValueEnum};
use std::fmt;

#[derive(Parser)]
#[command(name = "skop")]
#[command(version = "1.0")]
#[command(about = "Skill Manager for Codex, Opencode, and Antigravity")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Add a marketplace and install skills (interactive or single-skill non-interactive)
    #[command(
        long_about = "Add a marketplace and install skills.\n\nInteractive:\n  skop add <OWNER/REPO>\n\nNon-interactive single skill:\n  skop add <SKILL>@<OWNER/REPO> --target <codex|opencode|antigravity|all>\n\nNotes:\n  --target is required when using <SKILL>@<OWNER/REPO>.\n  --target can be used with <OWNER/REPO> to skip the target selection UI."
    )]
    Add {
        /// Show what would be installed without writing files
        #[arg(long)]
        dry_run: bool,

        /// Enable verbose logging
        #[arg(long)]
        verbose: bool,

        /// Maximum recursion depth when resolving nested marketplaces (default: 1)
        #[arg(long, default_value_t = 1)]
        max_depth: usize,

        /// Non-interactive target (codex, opencode, antigravity, all) when using <skill>@<owner>/<repo>
        #[arg(long)]
        target: Option<Target>,

        /// Repository owner/name (e.g. owner/repo) or <skill>@<owner>/<repo>
        repo: String,
    },
    /// Remove installed skills interactively
    Remove,
    /// List installed skills
    List,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum Target {
    Codex,
    Opencode,
    Antigravity,
    All,
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Target::Codex => write!(f, "codex"),
            Target::Opencode => write!(f, "opencode"),
            Target::Antigravity => write!(f, "antigravity"),
            Target::All => write!(f, "all"),
        }
    }
}

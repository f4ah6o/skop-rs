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
    /// Add a marketplace and install skills
    Add {
        /// Target environment (codex, opencode, antigravity)
        #[arg(long, value_enum)]
        target: Target,

        /// Repository owner/name (e.g. owner/repo)
        repo: String,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum Target {
    Codex,
    Opencode,
    Antigravity,
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Target::Codex => write!(f, "codex"),
            Target::Opencode => write!(f, "opencode"),
            Target::Antigravity => write!(f, "antigravity"),
        }
    }
}

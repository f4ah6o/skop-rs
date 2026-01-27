use crate::cli::Target;
use std::env;
use std::path::PathBuf;

pub fn get_skills_dir(target: Target) -> PathBuf {
    let current_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    match target {
        Target::Codex => current_dir.join(".codex/skills"),
        Target::Opencode => current_dir.join(".opencode/skills"),
        Target::Antigravity => current_dir.join(".agent/skills"),
        Target::All => unreachable!("Target::All should be handled before resolving a skills dir"),
    }
}

pub fn get_marketplace_url(repo: &str) -> String {
    // Assuming github.com and main branch for now, as per typical conventions unless specified otherwise
    // Real implementation might need to be smarter about branches (main/master) or use API.
    // Spec says: "repo": "owner/repo" in marketplace.json for github source.
    // For the marketplace file itself:
    // "Users add your marketplace with /plugin marketplace add owner/repo" -> implicitly looks for .claude-plugin/marketplace.json
    format!(
        "https://raw.githubusercontent.com/{}/main/.claude-plugin/marketplace.json",
        repo
    )
}

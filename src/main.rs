mod cli;
mod model;
mod util;

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use cli::{Cli, Commands, Target};
use crossterm::{cursor, event, execute, terminal};
use log::{info, warn};
use model::{Marketplace, PluginSource, SourceDefinition};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Serialize, Deserialize)]
struct PluginInstallMetadata {
    version: Option<String>,
    skills: Vec<String>,
}

#[derive(Clone, Debug)]
struct SkillEntry {
    name: String,
    path: PathBuf,
    target: Target,
}

#[derive(Clone, Copy)]
struct InstallOptions {
    dry_run: bool,
    max_depth: usize,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    init_logger(&cli);

    match cli.command {
        Commands::Add {
            target,
            dry_run,
            verbose: _,
            max_depth,
            repo,
        } => {
            let options = InstallOptions { dry_run, max_depth };
            handle_add(target, &repo, options)?;
        }
        Commands::Remove => {
            handle_remove()?;
        }
    }

    Ok(())
}

fn init_logger(cli: &Cli) {
    let default_level = match cli.command {
        Commands::Add { verbose, .. } => {
            if verbose {
                "info"
            } else {
                "warn"
            }
        }
        Commands::Remove => "warn",
    };
    let env = env_logger::Env::default().default_filter_or(default_level);
    let _ = env_logger::Builder::from_env(env).try_init();
}

fn handle_add(target: Target, repo: &str, options: InstallOptions) -> Result<()> {
    let skills_dir = util::get_skills_dir(target);
    if options.dry_run {
        println!("Dry run: no files will be modified.");
    } else {
        fs::create_dir_all(&skills_dir).context("Failed to create skills directory")?;
        fs::create_dir_all(skills_dir.join(".skop")).context("Failed to create metadata dir")?;
    }

    let url = util::get_marketplace_url(repo);
    info!("Fetching marketplace from {}", url);

    let resp = reqwest::blocking::get(&url)?;
    if !resp.status().is_success() {
        return Err(anyhow!(
            "Failed to fetch marketplace: status {}",
            resp.status()
        ));
    }

    let marketplace: Marketplace = resp.json()?;
    info!("Found marketplace: {}", marketplace.name);
    let plugin_root = marketplace
        .metadata
        .as_ref()
        .and_then(|metadata| metadata.plugin_root.as_deref());

    for plugin in marketplace.plugins {
        let metadata = read_plugin_metadata(&skills_dir, &plugin.name);
        let should_install = should_install_plugin(&plugin, metadata.as_ref());

        if !should_install {
            info!("Plugin {} is up to date.", plugin.name);
            continue;
        }

        if !options.dry_run {
            remove_legacy_plugin_dir(&skills_dir, &plugin.name)?;
            if let Some(existing) = &metadata {
                remove_installed_skills(&skills_dir, &plugin.name, existing)?;
            }
        }

        if options.dry_run {
            println!("Plugin: {}", plugin.name);
            println!("  marketplace.json: present");
            if !should_install {
                println!("  status: up to date");
            } else {
                println!("  status: would install/update");
            }
        }

        let installed_skills =
            install_plugin(&plugin, &skills_dir, repo, plugin_root, options)?;

        if options.dry_run {
            println!(
                "  skills: {}",
                if installed_skills.is_empty() {
                    "none".to_string()
                } else {
                    installed_skills.join(", ")
                }
            );
            continue;
        }

        let new_metadata = PluginInstallMetadata {
            version: plugin.version.clone(),
            skills: installed_skills.clone(),
        };
        write_plugin_metadata(&skills_dir, &plugin.name, &new_metadata)?;
        info!(
            "Installed {} ({} skill(s))",
            plugin.name,
            installed_skills.len()
        );
    }

    Ok(())
}

fn handle_remove() -> Result<()> {
    let entries = collect_installed_skills()?;
    if entries.is_empty() {
        println!("No skills found to remove.");
        return Ok(());
    }

    let selected = interactive_select_skills(&entries)?;
    if selected.is_empty() {
        println!("No skills selected.");
        return Ok(());
    }

    println!("Selected skills:");
    for entry in &selected {
        println!("  {} ({})", entry.name, entry.target);
    }

    if !confirm_removal(selected.len())? {
        println!("Cancelled.");
        return Ok(());
    }

    let mut removed_by_dir: HashMap<PathBuf, HashSet<String>> = HashMap::new();
    for entry in &selected {
        if entry.path.exists() {
            fs::remove_dir_all(&entry.path).with_context(|| {
                format!("Failed to remove skill directory {}", entry.path.display())
            })?;
        }
        let skills_dir = util::get_skills_dir(entry.target);
        removed_by_dir
            .entry(skills_dir)
            .or_default()
            .insert(entry.name.clone());
    }

    for (skills_dir, removed) in removed_by_dir {
        cleanup_metadata(&skills_dir, &removed)?;
    }

    println!("Removed {} skill(s).", selected.len());
    Ok(())
}

fn collect_installed_skills() -> Result<Vec<SkillEntry>> {
    let mut entries = Vec::new();
    for target in [Target::Codex, Target::Opencode, Target::Antigravity] {
        let skills_dir = util::get_skills_dir(target);
        if !skills_dir.exists() {
            continue;
        }
        for entry in fs::read_dir(&skills_dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name();
            let Some(name_str) = name.to_str() else { continue };
            if name_str == ".skop" {
                continue;
            }
            if entry.file_type()?.is_dir() && path.join("SKILL.md").is_file() {
                entries.push(SkillEntry {
                    name: name_str.to_string(),
                    path,
                    target,
                });
            }
        }
    }
    entries.sort_by(|a, b| a.name.cmp(&b.name).then(a.target.cmp(&b.target)));
    Ok(entries)
}

fn interactive_select_skills(entries: &[SkillEntry]) -> Result<Vec<SkillEntry>> {
    let mut selected = vec![false; entries.len()];
    let mut index = 0usize;
    let mut stdout = io::stdout();
    let _guard = RawModeGuard::new()?;

    loop {
        render_skill_list(&mut stdout, entries, &selected, index)?;
        match event::read()? {
            event::Event::Key(key) => match key.code {
                event::KeyCode::Char('q') | event::KeyCode::Esc => {
                    execute!(stdout, terminal::Clear(terminal::ClearType::All))?;
                    return Ok(Vec::new());
                }
                event::KeyCode::Up => {
                    if index > 0 {
                        index -= 1;
                    }
                }
                event::KeyCode::Down => {
                    if index + 1 < entries.len() {
                        index += 1;
                    }
                }
                event::KeyCode::Char(' ') => {
                    if let Some(state) = selected.get_mut(index) {
                        *state = !*state;
                    }
                }
                event::KeyCode::Enter => break,
                _ => {}
            },
            _ => {}
        }
    }

    execute!(stdout, terminal::Clear(terminal::ClearType::All))?;
    let chosen: Vec<SkillEntry> = entries
        .iter()
        .cloned()
        .zip(selected.into_iter())
        .filter_map(|(entry, is_selected)| if is_selected { Some(entry) } else { None })
        .collect();
    Ok(chosen)
}

fn render_skill_list(
    stdout: &mut io::Stdout,
    entries: &[SkillEntry],
    selected: &[bool],
    index: usize,
) -> Result<()> {
    execute!(stdout, terminal::Clear(terminal::ClearType::All))?;
    execute!(stdout, cursor::MoveTo(0, 0))?;
    writeln!(
        stdout,
        "Select skills to remove (space: toggle, ↑/↓: move, enter: confirm, q: quit)"
    )?;
    for (idx, entry) in entries.iter().enumerate() {
        let cursor = if idx == index { ">" } else { " " };
        let mark = if selected.get(idx).copied().unwrap_or(false) {
            "x"
        } else {
            " "
        };
        writeln!(stdout, "{} [{}] {} ({})", cursor, mark, entry.name, entry.target)?;
    }
    stdout.flush()?;
    Ok(())
}

struct RawModeGuard;

impl RawModeGuard {
    fn new() -> Result<Self> {
        terminal::enable_raw_mode()?;
        Ok(Self)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
    }
}

fn confirm_removal(count: usize) -> Result<bool> {
    print!("Remove {} skill(s)? (y/N): ", count);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(matches!(input.trim(), "y" | "Y"))
}

fn should_install_plugin(
    plugin: &model::PluginEntry,
    metadata: Option<&PluginInstallMetadata>,
) -> bool {
    let Some(metadata) = metadata else {
        info!("Installing new plugin: {}", plugin.name);
        return true;
    };

    match &plugin.version {
        Some(new_version) => match &metadata.version {
            Some(curr_version) => {
                match (
                    semver::Version::parse(curr_version),
                    semver::Version::parse(new_version),
                ) {
                    (Ok(v_curr), Ok(v_new)) => {
                        if v_new > v_curr {
                            info!(
                                "Updating {} from {} to {}",
                                plugin.name, curr_version, new_version
                            );
                            true
                        } else {
                            info!("Plugin {} is up to date ({}).", plugin.name, curr_version);
                            false
                        }
                    }
                    _ => {
                        warn!(
                            "Version parse failed for {}, reinstalling to be safe.",
                            plugin.name
                        );
                        true
                    }
                }
            }
            None => {
                info!(
                    "Plugin {} exists but no version in metadata, updating.",
                    plugin.name
                );
                true
            }
        },
        None => {
            info!(
                "Plugin {} has no version in marketplace, updating.",
                plugin.name
            );
            true
        }
    }
}

fn remove_legacy_plugin_dir(skills_dir: &Path, plugin_name: &str) -> Result<()> {
    let legacy_dir = skills_dir.join(plugin_name);
    let legacy_manifest = legacy_dir.join(".claude-plugin/plugin.json");
    if legacy_manifest.exists() {
        fs::remove_dir_all(&legacy_dir)
            .with_context(|| format!("Failed to remove legacy plugin dir: {}", plugin_name))?;
    }
    Ok(())
}

fn read_plugin_metadata(skills_dir: &Path, plugin_name: &str) -> Option<PluginInstallMetadata> {
    let path = plugin_metadata_path(skills_dir, plugin_name);
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn write_plugin_metadata(
    skills_dir: &Path,
    plugin_name: &str,
    metadata: &PluginInstallMetadata,
) -> Result<()> {
    let path = plugin_metadata_path(skills_dir, plugin_name);
    let content = serde_json::to_string_pretty(metadata)?;
    fs::write(path, content).context("Failed to write plugin metadata")
}

fn plugin_metadata_path(skills_dir: &Path, plugin_name: &str) -> PathBuf {
    skills_dir.join(".skop").join(format!("{}.json", plugin_name))
}

fn cleanup_metadata(skills_dir: &Path, removed_skills: &HashSet<String>) -> Result<()> {
    let meta_dir = skills_dir.join(".skop");
    if !meta_dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(&meta_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(_) => continue,
        };
        let mut metadata: PluginInstallMetadata = match serde_json::from_str(&content) {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };

        let before = metadata.skills.len();
        metadata
            .skills
            .retain(|skill| !removed_skills.contains(skill));
        if metadata.skills.len() == before {
            continue;
        }
        if metadata.skills.is_empty() {
            let _ = fs::remove_file(&path);
        } else {
            let updated = serde_json::to_string_pretty(&metadata)?;
            fs::write(&path, updated)?;
        }
    }

    Ok(())
}

fn remove_installed_skills(
    skills_dir: &Path,
    plugin_name: &str,
    metadata: &PluginInstallMetadata,
) -> Result<()> {
    for skill in &metadata.skills {
        let skill_dir = skills_dir.join(skill);
        if skill_dir.exists() {
            fs::remove_dir_all(&skill_dir).with_context(|| {
                format!("Failed to remove existing skill {} for {}", skill, plugin_name)
            })?;
        }
    }
    let meta_path = plugin_metadata_path(skills_dir, plugin_name);
    if meta_path.exists() {
        fs::remove_file(meta_path).context("Failed to remove old metadata")?;
    }
    Ok(())
}

fn install_plugin(
    plugin: &model::PluginEntry,
    skills_dir: &Path,
    marketplace_repo: &str,
    plugin_root: Option<&str>,
    options: InstallOptions,
) -> Result<Vec<String>> {
    let mut visited = HashSet::new();
    install_plugin_recursive(
        plugin,
        skills_dir,
        marketplace_repo,
        plugin_root,
        0,
        &mut visited,
        options,
    )
}

fn install_plugin_recursive(
    plugin: &model::PluginEntry,
    skills_dir: &Path,
    marketplace_repo: &str,
    plugin_root: Option<&str>,
    depth: usize,
    visited: &mut HashSet<String>,
    options: InstallOptions,
) -> Result<Vec<String>> {
    if depth > options.max_depth {
        return handle_missing_skills(
            options,
            &format!(
                "Maximum recursion depth exceeded while resolving {}",
                plugin.name
            ),
        );
    }

    let (git_url, subpath, git_ref) = resolve_plugin_url(plugin, marketplace_repo, plugin_root);
    let visit_key = match &git_ref {
        Some(r) => format!("{}#{}", git_url, r),
        None => git_url.clone(),
    };
    if !visited.insert(visit_key) {
        return handle_missing_skills(
            options,
            &format!(
                "Detected recursive plugin source loop while resolving {}",
                plugin.name
            ),
        );
    }

    let temp_dir = tempfile::Builder::new().prefix("skop_install").tempdir()?;
    info!("Cloning {} ...", git_url);
    if options.dry_run {
        let indent = "  ".repeat(depth + 1);
        println!("{indent}repo: {}", git_url);
        if let Some(subpath) = &subpath {
            println!("{indent}source path: {}", subpath);
        }
    }

    let mut cmd = Command::new("git");
    cmd.arg("clone").arg("--depth").arg("1");
    if let Some(r) = &git_ref {
        cmd.arg("--branch").arg(r);
    }
    cmd.arg(&git_url).arg(temp_dir.path());

    let output = cmd.output().context("Failed to execute git clone")?;
    if !output.status.success() {
        return Err(anyhow!(
            "Git clone failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let repo_root = temp_dir.path().to_path_buf();
    let source_path = if let Some(p) = subpath {
        repo_root.join(p)
    } else {
        repo_root.clone()
    };

    if source_path.exists() {
        let skill_paths = discover_skill_dirs(&source_path, plugin)?;
        if !skill_paths.is_empty() {
            if options.dry_run {
                let indent = "  ".repeat(depth + 1);
                println!("{indent}skills detected: {}", format_skill_names(&skill_paths));
                return Ok(extract_skill_names(skill_paths));
            }
            return install_skills_from_paths(skills_dir, skill_paths);
        }
    }

    if let Some(marketplace) = read_marketplace_from_repo(&repo_root) {
        if options.dry_run {
            let indent = "  ".repeat(depth + 1);
            println!("{indent}marketplace.json: found");
        }
        if let Some(nested_plugin) = marketplace.plugins.iter().find(|p| p.name == plugin.name) {
            if options.dry_run {
                let indent = "  ".repeat(depth + 1);
                println!(
                    "{indent}recursive: using marketplace entry for {}",
                    plugin.name
                );
            }
            let nested_root = marketplace
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.plugin_root.as_deref());
            return install_from_marketplace_entry(
                nested_plugin,
                skills_dir,
                &repo_root,
                &git_url,
                nested_root,
                depth + 1,
                visited,
                options,
            );
        }
        if options.dry_run {
            let indent = "  ".repeat(depth + 1);
            println!(
                "{indent}marketplace.json: no matching plugin entry for {}",
                plugin.name
            );
        }
    } else if options.dry_run {
        let indent = "  ".repeat(depth + 1);
        println!("{indent}marketplace.json: absent");
    }

    handle_missing_skills(
        options,
        &format!(
            "No skills found for plugin {} at {:?}",
            plugin.name, source_path
        ),
    )
}

fn install_from_marketplace_entry(
    plugin: &model::PluginEntry,
    skills_dir: &Path,
    repo_root: &Path,
    repo_url: &str,
    plugin_root: Option<&str>,
    depth: usize,
    visited: &mut HashSet<String>,
    options: InstallOptions,
) -> Result<Vec<String>> {
    match &plugin.source {
        PluginSource::Path(path) => {
            let resolved_path = apply_plugin_root(path, plugin_root);
            let source_path = repo_root.join(resolved_path);
            if !source_path.exists() {
                return handle_missing_skills(
                    options,
                    &format!("Plugin source path does not exist: {:?}", source_path),
                );
            }

            let skill_paths = discover_skill_dirs(&source_path, plugin)?;
            if skill_paths.is_empty() {
                return handle_missing_skills(
                    options,
                    &format!(
                        "No skills found for plugin {} at {:?}",
                        plugin.name, source_path
                    ),
                );
            }
            if options.dry_run {
                let indent = "  ".repeat(depth + 1);
                println!("{indent}marketplace entry: path");
                println!("{indent}skills detected: {}", format_skill_names(&skill_paths));
                return Ok(extract_skill_names(skill_paths));
            }
            install_skills_from_paths(skills_dir, skill_paths)
        }
        PluginSource::Object(_) => {
            if options.dry_run {
                let indent = "  ".repeat(depth + 1);
                println!("{indent}recursive: following source object");
            }
            install_plugin_recursive(
                plugin,
                skills_dir,
                repo_url,
                plugin_root,
                depth,
                visited,
                options,
            )
        }
    }
}

fn handle_missing_skills(options: InstallOptions, message: &str) -> Result<Vec<String>> {
    if options.dry_run {
        println!("  {}", message);
        return Ok(Vec::new());
    }
    Err(anyhow!(message.to_string()))
}

fn format_skill_names(skill_paths: &[PathBuf]) -> String {
    let names: Vec<String> = skill_paths
        .iter()
        .filter_map(|path| path.file_name().and_then(|name| name.to_str()))
        .map(|name| name.to_string())
        .collect();
    if names.is_empty() {
        "none".to_string()
    } else {
        names.join(", ")
    }
}

fn extract_skill_names(skill_paths: Vec<PathBuf>) -> Vec<String> {
    skill_paths
        .into_iter()
        .filter_map(|path| path.file_name().and_then(|name| name.to_str()).map(|s| s.to_string()))
        .collect()
}

fn install_skills_from_paths(skills_dir: &Path, skill_paths: Vec<PathBuf>) -> Result<Vec<String>> {
    let mut installed_skills = Vec::new();
    for skill_path in skill_paths {
        let Some(skill_name) = skill_path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.to_string())
        else {
            continue;
        };

        let dest = skills_dir.join(&skill_name);
        if dest.exists() {
            fs::remove_dir_all(&dest).with_context(|| {
                format!("Failed to remove existing skill dir for {}", skill_name)
            })?;
        }
        copy_dir_all(&skill_path, &dest)?;
        installed_skills.push(skill_name);
    }

    Ok(installed_skills)
}

fn read_marketplace_from_repo(repo_root: &Path) -> Option<Marketplace> {
    let path = repo_root.join(".claude-plugin/marketplace.json");
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn resolve_plugin_url(
    plugin: &model::PluginEntry,
    marketplace_repo: &str,
    plugin_root: Option<&str>,
) -> (String, Option<String>, Option<String>) {
    let base_repo_url = resolve_marketplace_repo_url(marketplace_repo);
    let get_override_url = |plugin: &model::PluginEntry| -> Option<String> {
        if let Some(author) = &plugin.author {
            if let Some(url) = &author.url {
                if url.starts_with("http") || url.starts_with("git@") {
                    return Some(url.clone());
                } else {
                    return Some(format!("https://github.com/{}.git", url));
                }
            }
        }
        if let Some(repo) = &plugin.repository {
            if repo.starts_with("http") || repo.starts_with("git@") {
                return Some(repo.clone());
            } else {
                return Some(format!("https://github.com/{}.git", repo));
            }
        }
        None
    };

    match &plugin.source {
        PluginSource::Path(p) => {
            let repo_url = get_override_url(plugin).unwrap_or_else(|| base_repo_url.clone());
            let resolved_path = apply_plugin_root(p, plugin_root);

            (repo_url, Some(resolved_path), None)
        }
        PluginSource::Object(def) => match def {
            SourceDefinition::Github {
                repo,
                ref_,
                sha: _,
            } => {
                // For explicit Github source, use the defined repo, ignoring overrides
                (format!("https://github.com/{}.git", repo), None, ref_.clone())
            }
            SourceDefinition::Url {
                url,
                ref_,
                sha: _,
            } => {
                // For explicit URL source, use the defined URL, ignoring overrides
                (url.clone(), None, ref_.clone())
            }
        },
    }
}

fn resolve_marketplace_repo_url(marketplace_repo: &str) -> String {
    if marketplace_repo.starts_with("http") || marketplace_repo.starts_with("git@") {
        marketplace_repo.to_string()
    } else {
        format!("https://github.com/{}.git", marketplace_repo)
    }
}

fn apply_plugin_root(path: &str, plugin_root: Option<&str>) -> String {
    let Some(root) = plugin_root else {
        return path.to_string();
    };

    if is_explicit_path(path) {
        return path.to_string();
    }

    Path::new(root)
        .join(path)
        .to_string_lossy()
        .to_string()
}

fn is_explicit_path(path: &str) -> bool {
    path.starts_with("./") || path.starts_with("../") || path.starts_with('/')
}

fn discover_skill_dirs(
    plugin_root: &Path,
    plugin: &model::PluginEntry,
) -> Result<Vec<PathBuf>> {
    let mut skill_paths = Vec::new();
    if let Some(paths) = extract_skill_paths(plugin) {
        skill_paths.extend(collect_skills_from_candidates(plugin_root, &paths)?);
    }

    if skill_paths.is_empty() {
        skill_paths.extend(collect_skills_from_candidates(
            plugin_root,
            &["skills".to_string()],
        )?);
    }

    if skill_paths.is_empty() {
        skill_paths.extend(collect_skills_from_candidates(
            plugin_root,
            &[".".to_string()],
        )?);
    }

    if skill_paths.is_empty() && plugin_root.join("SKILL.md").is_file() {
        skill_paths.push(plugin_root.to_path_buf());
    }

    let mut seen = HashSet::new();
    let mut unique = Vec::new();
    for path in skill_paths {
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if seen.insert(name.to_string()) {
            unique.push(path);
        } else {
            warn!("Duplicate skill name {}, skipping {:?}", name, path);
        }
    }

    Ok(unique)
}

fn extract_skill_paths(plugin: &model::PluginEntry) -> Option<Vec<String>> {
    if let Some(value) = plugin.extra.get("skills") {
        return extract_paths_from_value(value);
    }
    if let Some(value) = plugin.extra.get("agents") {
        return extract_paths_from_value(value);
    }
    None
}

fn extract_paths_from_value(value: &Value) -> Option<Vec<String>> {
    match value {
        Value::String(path) => Some(vec![path.to_string()]),
        Value::Array(entries) => {
            let paths: Vec<String> = entries
                .iter()
                .filter_map(|entry| entry.as_str().map(|s| s.to_string()))
                .collect();
            if paths.is_empty() { None } else { Some(paths) }
        }
        Value::Object(map) => {
            if let Some(path) = map.get("path").and_then(|entry| entry.as_str()) {
                return Some(vec![path.to_string()]);
            }
            if let Some(paths) = map.get("paths").and_then(|entry| entry.as_array()) {
                let paths: Vec<String> = paths
                    .iter()
                    .filter_map(|entry| entry.as_str().map(|s| s.to_string()))
                    .collect();
                if !paths.is_empty() {
                    return Some(paths);
                }
            }
            None
        }
        _ => None,
    }
}

fn collect_skills_from_candidates(
    plugin_root: &Path,
    candidates: &[String],
) -> Result<Vec<PathBuf>> {
    let mut skill_paths = Vec::new();
    for candidate in candidates {
        let candidate_path = plugin_root.join(candidate);
        if candidate_path.is_file() {
            if candidate_path
                .file_name()
                .and_then(|name| name.to_str())
                == Some("SKILL.md")
            {
                if let Some(parent) = candidate_path.parent() {
                    skill_paths.push(parent.to_path_buf());
                }
            }
            continue;
        }

        if !candidate_path.is_dir() {
            continue;
        }

        if candidate_path.join("SKILL.md").is_file() {
            skill_paths.push(candidate_path);
            continue;
        }

        for entry in fs::read_dir(&candidate_path)? {
            let entry = entry?;
            let path = entry.path();
            if entry.file_type()?.is_dir() {
                if path.join("SKILL.md").is_file() {
                    skill_paths.push(path);
                }
            } else if path
                .file_name()
                .and_then(|name| name.to_str())
                == Some("SKILL.md")
            {
                if let Some(parent) = path.parent() {
                    skill_paths.push(parent.to_path_buf());
                }
            }
        }
    }

    Ok(skill_paths)
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let path = entry.path();
        let name = entry.file_name();
        let dst_path = dst.join(name);

        if ty.is_dir() {
            // Skip .git directory
            if path.file_name().and_then(|s| s.to_str()) == Some(".git") {
                continue;
            }
            copy_dir_all(&path, &dst_path)?;
        } else {
            fs::copy(path, dst_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Author, PluginEntry, PluginSource, SourceDefinition};
    use serde_json::json;
    use std::collections::HashMap;

    fn create_dummy_plugin(
        source: PluginSource,
        author_url: Option<String>,
        repository: Option<String>,
    ) -> PluginEntry {
        PluginEntry {
            name: "test-plugin".to_string(),
            source,
            description: None,
            version: None,
            repository,
            author: Some(Author {
                name: Some("Test Author".to_string()),
                email: None,
                url: author_url,
            }),
            extra: HashMap::new(),
        }
    }

    #[test]
    fn test_resolve_path_defaults_to_marketplace() {
        let source = PluginSource::Path("./skills/test".to_string());
        let plugin = create_dummy_plugin(source, None, None);
        let (url, subpath, _) = resolve_plugin_url(&plugin, "owner/marketplace", None);

        assert_eq!(url, "https://github.com/owner/marketplace.git");
        assert_eq!(subpath, Some("./skills/test".to_string()));
    }

    #[test]
    fn test_resolve_path_applies_plugin_root() {
        let source = PluginSource::Path("formatter".to_string());
        let plugin = create_dummy_plugin(source, None, None);
        let (url, subpath, _) =
            resolve_plugin_url(&plugin, "owner/marketplace", Some("./plugins"));

        assert_eq!(url, "https://github.com/owner/marketplace.git");
        assert_eq!(subpath, Some("./plugins/formatter".to_string()));
    }

    #[test]
    fn test_resolve_path_does_not_double_prefix() {
        let source = PluginSource::Path("./plugins/formatter".to_string());
        let plugin = create_dummy_plugin(source, None, None);
        let (_, subpath, _) = resolve_plugin_url(&plugin, "owner/marketplace", Some("./plugins"));

        assert_eq!(subpath, Some("./plugins/formatter".to_string()));
    }

    #[test]
    fn test_resolve_path_with_marketplace_url() {
        let source = PluginSource::Path("./skills/test".to_string());
        let plugin = create_dummy_plugin(source, None, None);
        let (url, subpath, _) = resolve_plugin_url(
            &plugin,
            "https://github.com/example/repo.git",
            None,
        );

        assert_eq!(url, "https://github.com/example/repo.git");
        assert_eq!(subpath, Some("./skills/test".to_string()));
    }

    #[test]
    fn test_resolve_path_uses_author_url_override() {
        let source = PluginSource::Path("./skills/test".to_string());
        let plugin = create_dummy_plugin(source, Some("other/repo".to_string()), None);
        let (url, subpath, _) = resolve_plugin_url(&plugin, "owner/marketplace", None);

        assert_eq!(url, "https://github.com/other/repo.git");
        assert_eq!(subpath, Some("./skills/test".to_string()));
    }

    #[test]
    fn test_resolve_path_uses_repository_override() {
        let source = PluginSource::Path("./skills/test".to_string());
        let plugin = create_dummy_plugin(
            source,
            None,
            Some("https://github.com/repo/over".to_string()),
        );
        let (url, subpath, _) = resolve_plugin_url(&plugin, "owner/marketplace", None);

        assert_eq!(url, "https://github.com/repo/over");
        assert_eq!(subpath, Some("./skills/test".to_string()));
    }

    #[test]
    fn test_resolve_github_object_ignores_override() {
        let source = PluginSource::Object(SourceDefinition::Github {
            repo: "original/repo".to_string(),
            ref_: None,
            sha: None,
        });
        let plugin = create_dummy_plugin(source, Some("override/repo".to_string()), None);
        let (url, _, _) = resolve_plugin_url(&plugin, "owner/marketplace", None);

        assert_eq!(url, "https://github.com/original/repo.git");
    }

    #[test]
    fn test_resolve_github_object_defaults_to_source_repo() {
        let source = PluginSource::Object(SourceDefinition::Github {
            repo: "original/repo".to_string(),
            ref_: None,
            sha: None,
        });
        let plugin = create_dummy_plugin(source, None, None);
        let (url, _, _) = resolve_plugin_url(&plugin, "owner/marketplace", None);

        assert_eq!(url, "https://github.com/original/repo.git");
    }

    #[test]
    fn test_discover_skill_dirs_from_skills_folder() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let skill_dir = root.join("skills/review");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "skill").unwrap();

        let plugin = create_dummy_plugin(PluginSource::Path(".".to_string()), None, None);
        let skills = discover_skill_dirs(root, &plugin).unwrap();

        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].file_name().unwrap(), "review");
    }

    #[test]
    fn test_discover_skill_dirs_from_custom_paths() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let skill_dir = root.join("custom-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "skill").unwrap();

        let mut plugin = create_dummy_plugin(PluginSource::Path(".".to_string()), None, None);
        plugin.extra.insert("skills".to_string(), json!("custom-skill"));

        let skills = discover_skill_dirs(root, &plugin).unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].file_name().unwrap(), "custom-skill");
    }

    #[test]
    fn test_discover_skill_dirs_from_root_skill() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        fs::write(root.join("SKILL.md"), "skill").unwrap();

        let plugin = create_dummy_plugin(PluginSource::Path(".".to_string()), None, None);
        let skills = discover_skill_dirs(root, &plugin).unwrap();

        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0], root);
    }

    #[test]
    fn test_discover_skill_dirs_from_root_subdirectories() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let skill_dir = root.join("moonbit-agent-guide");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "skill").unwrap();

        let plugin = create_dummy_plugin(PluginSource::Path(".".to_string()), None, None);
        let skills = discover_skill_dirs(root, &plugin).unwrap();

        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].file_name().unwrap(), "moonbit-agent-guide");
    }
}

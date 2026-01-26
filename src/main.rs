mod cli;
mod model;
mod util;

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use cli::{Cli, Commands, Target};
use log::{info, warn};
use model::{Marketplace, PluginSource, SourceDefinition};
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(serde::Deserialize)]
struct PluginManifest {
    version: String,
}

fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Add { target, repo } => {
            handle_add(target, &repo)?;
        }
    }

    Ok(())
}

fn handle_add(target: Target, repo: &str) -> Result<()> {
    let skills_dir = util::get_skills_dir(target);
    fs::create_dir_all(&skills_dir).context("Failed to create skills directory")?;

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

    for plugin in marketplace.plugins {
        let plugin_dir = skills_dir.join(&plugin.name);

        let should_install = if plugin_dir.exists() {
            // Check version
            let manifest_path = plugin_dir.join(".claude-plugin/plugin.json");
            if let Ok(content) = fs::read_to_string(&manifest_path) {
                if let Ok(manifest) = serde_json::from_str::<PluginManifest>(&content) {
                    if let Some(new_version) = &plugin.version {
                        if let (Ok(v_curr), Ok(v_new)) = (
                            semver::Version::parse(&manifest.version),
                            semver::Version::parse(new_version),
                        ) {
                            if v_new > v_curr {
                                info!(
                                    "Updating {} from {} to {}",
                                    plugin.name, manifest.version, new_version
                                );
                                true
                            } else {
                                info!(
                                    "Plugin {} is up to date ({}).",
                                    plugin.name, manifest.version
                                );
                                false
                            }
                        } else {
                            warn!(
                                "Version parse failed for {}, reinstalling to be safe.",
                                plugin.name
                            );
                            true
                        }
                    } else {
                        // Remote has no version specified? Re-install or ignore?
                        // Let's assume re-install to be safe/latest.
                        info!(
                            "Plugin {} exists but no version in marketplace, updating.",
                            plugin.name
                        );
                        true
                    }
                } else {
                    warn!(
                        "Could not parse manifest for {}, reinstalling.",
                        plugin.name
                    );
                    true
                }
            } else {
                warn!(
                    "Plugin directory exists but no manifest for {}, reinstalling.",
                    plugin.name
                );
                true
            }
        } else {
            info!("Installing new plugin: {}", plugin.name);
            true
        };

        if should_install {
            if plugin_dir.exists() {
                fs::remove_dir_all(&plugin_dir).context("Failed to remove old plugin version")?;
            }
            install_plugin(&plugin, &plugin_dir, repo)?;
            info!("Installed {}", plugin.name);
        }
    }

    Ok(())
}

fn install_plugin(plugin: &model::PluginEntry, dest: &Path, marketplace_repo: &str) -> Result<()> {
    let temp_dir = tempfile::Builder::new().prefix("skop_install").tempdir()?;

    let (git_url, subpath, git_ref) = match &plugin.source {
        PluginSource::Path(p) => {
            // It's relative to the marketplace repo
            (
                format!("https://github.com/{}.git", marketplace_repo),
                Some(p.clone()),
                None,
            )
        }
        PluginSource::Object(def) => match def {
            SourceDefinition::Github { repo, ref_, sha: _ } => (
                format!("https://github.com/{}.git", repo),
                None,
                ref_.clone(),
            ),
            SourceDefinition::Url { url, ref_, sha: _ } => (url.clone(), None, ref_.clone()),
        },
    };

    info!("Cloning {} ...", git_url);

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

    let source_path = if let Some(p) = subpath {
        temp_dir.path().join(p)
    } else {
        temp_dir.path().to_path_buf()
    };

    if !source_path.exists() {
        return Err(anyhow!(
            "Plugin source path does not exist: {:?}",
            source_path
        ));
    }

    // Move/Copy files
    // fs::rename doesn't work across filesystems, so copy recursively
    copy_dir_all(&source_path, dest)?;

    Ok(())
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

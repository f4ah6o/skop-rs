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

    // Determine the git URL and subpath logic
    // Logic priority:
    // 1. If explicit 'repository' field exists in PluginEntry, use that as the repo URL (unless overridden by source definitions which are more specific).
    //    Actually, 'repository' field is metadata. The 'source' field is the authority on WHERE to fetch.
    //    But the user says: "plugins... repository field has skill URL. This might differ from argument owner/repo"
    //    If the source is a relative Path, it implies it is inside the repository we are talking about.
    //    If the user meant that we should fallback or use 'repository' if 'source' is vague, that's one thing.
    //    But looking at the spec:
    //    "repository": "Source code repository URL" (optional metadata)
    //    "source": Where to fetch (Required).
    //
    //    If source is "github" object -> it has its own repo field.
    //    If source is "url" object -> it has its own url field.
    //    If source is string (Path) -> it is relative to the marketplace repo.
    //
    //    The user's request: "repository field... differs from argument <owner/repo>"
    //    This implies that if we are using the marketplace repo (because source is a Path), we should perhaps consider if 'repository' field points elsewhere?
    //    No, the spec says "For plugins in the same repository: source: ./plugins/my-plugin".
    //    So if source is a path, it MUST be in the marketplace repo.
    //
    //    Wait, maybe the user means: When source is a Path, the "marketplace repo" is defined by the argument <owner/repo> passed to CLI.
    //    BUT, if the marketplace.json itself was fetched from <owner/repo>, then relative paths are relative to THAT.
    //
    //    Let's re-read the issue: "repository field has skill URL... inconsistent with argument".
    //    Maybe the user means I should use `plugin.repository` if available when cloning?
    //    But `plugin.repository` is just a metadata link to the repo, it doesn't specify *how* to fetch (e.g. no ref/sha).
    //    `source` is the functional definition.
    //
    //    However, if the `source` is a relative path, we default to constructing the URL from `marketplace_repo`.
    //    If `plugin.repository` is present, should we use THAT instead of `marketplace_repo`?
    //    If `plugin.source` is a relative path like `./foo`, it implies it is physically located in the repo where `marketplace.json` is.
    //    If `plugin.repository` says `github.com/other/repo`, then `./foo` would not exist there unless it's a monorepo situation or misunderstanding.
    //
    //    Actually, looking at the spec example:
    //    "repository": "https://github.com/company/enterprise-plugin",
    //    "source": { "source": "github", "repo": "company/enterprise-plugin" }
    //
    //    If source is just `./plugins/my-plugin`, then it IS in the marketplace repo.
    //
    //    Perhaps the user is referring to the case where I constructed the URL:
    //    `format!("https://github.com/{}.git", marketplace_repo)`
    //
    //    If I am misinterpreting the `marketplace_repo` argument.
    //    The CLI arg is `repo` (owner/repo).
    //
    //    Ah, I think I see.
    //    If the user provides `skop add target owner/repo`, I assume `marketplace.json` is at `owner/repo`.
    //    And for relative paths, I clone `owner/repo`.
    //
    //    The user says: "plugins array... repository field has skill URL. This url might differ from argument owner/repo".
    //    This might simply mean: "Hey, check the `repository` field in the plugin entry. If it exists, use that as the clone URL instead of the CLI argument, UNLESS source specifically says otherwise?"
    //
    //    BUT `repository` is optional metadata. `source` is mandatory.
    //    If `source` is a `Path`, it is RELATIVE to the marketplace.
    //
    //    Let's assume the user wants me to fix the `PluginSource::Path` case.
    //    If `PluginSource::Path` is used, it MUST be relative to the marketplace repository.
    //    So my current logic: `format!("https://github.com/{}.git", marketplace_repo)` is correct according to spec for `Path`.
    //
    //    Maybe the user implies that for `SourceDefinition::Github`, I should be careful?
    //    `SourceDefinition::Github { repo, ... }` -> I use `repo` from the source object. This ignores CLI arg `marketplace_repo`. This is CORRECT.
    //
    //    Let's look at what I implemented:
    //
    //    ```rust
    //    match &plugin.source {
    //       PluginSource::Path(p) => {
    //           (format!("https://github.com/{}.git", marketplace_repo), Some(p.clone()), None)
    //       },
    //       PluginSource::Object(def) => match def {
    //           SourceDefinition::Github { repo, ref_, sha: _ } => {
    //               (format!("https://github.com/{}.git", repo), None, ref_.clone())
    //           },
    //           ...
    //    ```
    //
    //    This looks correct.
    //
    //    Wait, I might have misunderstood "repository field in plugin entry".
    //    `pub repository: Option<String>` in `PluginEntry` struct (which I just added).
    //
    //    If the user says "repository field has skill URL", maybe they mean I should prioritize it?
    //    But `source` is the source of truth.
    //
    //    Hypothesis: The user wants me to support a case where `source` is NOT provided or I should look at `repository`?
    //    No, `source` is required by spec.
    //
    //    Let's look at the feedback again carefully.
    //    "plugins array... repository field has skill URL. This url might differ from argument <owner/repo>."
    //
    //    If the user is talking about `SourceDefinition::Github`, my code uses `repo` from the source definition.
    //    `SourceDefinition::Github { repo, ... }` -> `repo` is used.
    //
    //    If the user is talking about `PluginSource::Path`, then it MUST be the same repo.
    //
    //    Is it possible the user is using a custom `repository` field in the JSON that holds the git clone URL, and `source` is just a path relative to THAT?
    //    The spec doesn't say that. The spec says `source` is "Where to fetch the plugin from".
    //
    //    Maybe the issue is that `marketplace_repo` passed to `install_plugin` is the CLI argument.
    //    For `PluginSource::Path`, I use `marketplace_repo`.
    //
    //    If I am processing a `marketplace.json` that was fetched from `owner/repo`, then `PluginSource::Path` refers to `owner/repo`.
    //
    //    Wait, if I am recursing or something? No.
    //
    //    Let's assume the user simply noticed that I might be using `marketplace_repo` (the CLI arg) in cases where I should be using the repo defined in the plugin's `source` object.
    //    But I AM using the repo from the `source` object for `Github` type.
    //
    //    Ah! Maybe the user means I should respect `plugin.repository` if it is present, as an override?
    //    Or maybe they mean for `PluginSource::Path`, if `plugin.repository` is set, use THAT as the base?
    //    The spec says: "repository: Source code repository URL".
    //    If I have:
    //    {
    //      "name": "foo",
    //      "source": "./skills/foo",
    //      "repository": "https://github.com/another/repo"
    //    }
    //    Does this mean fetch `./skills/foo` from `https://github.com/another/repo`?
    //    The spec says: "For plugins in the same repository... Relative paths only work when users add your marketplace via Git...".
    //    It strongly implies "same repository" means the marketplace repository.
    //
    //    HOWEVER, if the user explicitly asks for this fix, I should probably implement it:
    //    If `source` is a Path, AND `repository` field is present, use the `repository` URL as the clone target, and the path as relative to that.
    //
    //    Let's try that logic.

    let (git_url, subpath, git_ref) = match &plugin.source {
        PluginSource::Path(p) => {
            // If plugin has explicit repository metadata, usage it as the base for the relative path
            // Otherwise use the marketplace repo.
            let repo_url = if let Some(repo) = &plugin.repository {
                // If it's a full URL, use it. If it's owner/repo, format it.
                if repo.starts_with("http") || repo.starts_with("git@") {
                    repo.clone()
                } else {
                    format!("https://github.com/{}.git", repo)
                }
            } else {
                format!("https://github.com/{}.git", marketplace_repo)
            };

            (repo_url, Some(p.clone()), None)
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

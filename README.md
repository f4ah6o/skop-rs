# Skop - Skill Manager for AI Coding Agents

Skop is a CLI tool designed to manage skills for various AI coding agents, including Codex, Opencode, and Antigravity. It allows you to easily install and update skills defined in a Claude Plugin Marketplace.

## Features

- **Multi-Agent Support**: Install skills for Codex (`.codex/skills`), Opencode (`.opencode/skills`), and Antigravity (`.agent/skills`).
- **Marketplace Integration**: Consumes `marketplace.json` compatible with the Claude Plugin Marketplace specification.
- **Smart Updates**: Automatically checks versioning to update existing skills or install new ones.
- **Flexible Sources**: Supports skills hosted in the marketplace repository (relative paths) or external repositories (GitHub, Git URLs).

## Installation

Ensure you have Rust and Cargo installed.

```bash
cargo install --path .
```

## Usage

### Add a Marketplace

To add skills from a marketplace to a specific agent's environment:

```bash
skop add --target <TARGET> <OWNER/REPO>
```

- **TARGET**: The target agent environment. One of:
    - `codex`: Installs to `$CWD/.codex/skills`
    - `opencode`: Installs to `$CWD/.opencode/skills`
    - `antigravity`: Installs to `$CWD/.agent/skills`
- **OWNER/REPO**: The GitHub repository containing the `marketplace.json` file (e.g., `owner/my-marketplace`).
- **Options**:
    - `--dry-run`: Print what would be installed without writing files.
    - `--verbose`: Enable verbose logging.
    - `--max-depth <N>`: Maximum recursion depth when resolving nested marketplaces (default: 1).

### Remove Skills (Interactive)

```bash
skop remove
```

- Lists installed skills across all supported agent directories as `<skill name> (<agent>)`.
- Use arrow keys to move, space to toggle selection.
- Press Enter, then confirm with `y` to remove.

### Examples

Install skills for **Codex** from a marketplace:
```bash
skop add --target codex my-org/coding-skills
```

Install skills for **Opencode**:
```bash
skop add --target opencode community-skills/python-tools
```

## Marketplace Format

Skop expects the remote repository to contain a `.claude-plugin/marketplace.json` file following the [Claude Plugin Marketplace schema](https://code.claude.com/docs/ja/plugin-marketplaces).

Example `marketplace.json`:
```json
{
  "name": "my-skills",
  "owner": {
    "name": "My Team"
  },
  "plugins": [
    {
      "name": "lint-checker",
      "source": "./skills/lint-checker",
      "version": "1.0.0",
      "description": "A skill to run linters"
    },
    {
      "name": "external-tool",
      "source": {
        "source": "github",
        "repo": "another-owner/tool-repo"
      },
      "version": "2.1.0"
    }
  ]
}
```

## How it works

1. **Fetch**: Skop retrieves the `marketplace.json` from the specified GitHub repository.
2. **Resolve**: For each plugin, it determines the source repository.
    - If the source is a relative path (e.g., `./skills/foo`), it defaults to cloning the marketplace repository itself.
    - **Override**: If a plugin entry has a `repository` field, it will use that URL as the base for relative path sources.
    - If the source is an explicit object (GitHub/URL), it uses that definition.
    - If `metadata.pluginRoot` is set in the marketplace, relative paths (without `./`) are resolved against it.
3. **Check**: It compares the `version` in `marketplace.json` with the locally stored metadata.
4. **Install/Update**: If the plugin is new or has a higher version, Skop clones the repository (shallow clone), discovers skill folders (directories containing `SKILL.md`), and copies them into the agent's skill directory. It stores install metadata in `.skop/<plugin>.json`.
    - Skill discovery prefers `skills` or `agents` paths in the plugin entry when provided, otherwise it falls back to the conventional `skills/` layout.

## License

[MIT](LICENSE)

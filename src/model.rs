use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct Marketplace {
    pub name: String,
    pub owner: Owner,
    pub plugins: Vec<PluginEntry>,
    #[serde(default)]
    pub metadata: Option<Metadata>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Owner {
    pub name: String,
    #[serde(default)]
    pub email: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Metadata {
    pub description: Option<String>,
    pub version: Option<String>,
    #[serde(rename = "pluginRoot")]
    pub plugin_root: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PluginEntry {
    pub name: String,
    pub source: PluginSource,
    pub description: Option<String>,
    pub version: Option<String>,
    pub repository: Option<String>,
    // There are many other optional fields, we can add them as needed or use flattened HashMap for extras
    // For version comparison, 'version' is key.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PluginSource {
    Path(String),
    Object(SourceDefinition),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "source")]
#[serde(rename_all = "camelCase")] // "github", "url"
pub enum SourceDefinition {
    Github {
        repo: String,
        ref_: Option<String>, // "ref" is a reserved keyword in Rust (kinda, but safe as field identifier usually, but let's use ref_)
        sha: Option<String>,
    },
    Url {
        url: String,
        ref_: Option<String>,
        sha: Option<String>,
    },
    // The spec also mentions "npm" but says it's not fully implemented.
}

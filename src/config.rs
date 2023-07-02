use std::collections::HashMap;
use std::path::PathBuf;

use crate::{Request, Response, Test};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use walkdir::WalkDir;

/// Errors that can occur when managing the configuration.
#[derive(Debug, Error)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("yaml parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("path error: {0}")]
    Path(String),

    #[error("context not found: {0}")]
    ContextNotFound(String),
}

/// The configuration for the CLI.
#[derive(Default, Debug, Deserialize, Serialize)]
pub struct Config {
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub contexts: HashMap<String, HashMap<String, String>>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub requests: HashMap<String, Request>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub responses: HashMap<String, Response>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub tests: HashMap<String, Test>,
}

/// Result is a convenience type for config errors.
type Result<T> = std::result::Result<T, Error>;

impl Config {
    pub fn new(path: &str) -> Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        Ok(serde_yaml::from_str(&contents)?)
    }

    pub fn new_from_path(path: &PathBuf) -> Result<Self> {
        let mut cfg: Config = Config::default();
        // Loop through the path and only parse yaml files.
        for entry in WalkDir::new(path).follow_links(true) {
            let entry = entry.map_err(|e| Error::Path(e.to_string()))?;
            if entry.file_type().is_file() {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    if ext == "yaml" || ext == "yml" {
                        let c = Config::new(
                            path.to_str().ok_or(Error::Path("non-ascii path".into()))?,
                        )?;
                        cfg.merge(c);
                    }
                }
            }
        }
        Ok(cfg)
    }

    pub fn load_responses(&mut self, path: &PathBuf) -> Result<()> {
        for entry in WalkDir::new(path).follow_links(true) {
            let entry = entry.map_err(|e| Error::Path(e.to_string()))?;
            if entry.file_type().is_file() {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    if ext == "yaml" || ext == "yml" {
                        // Get the basename without extension.
                        let name = path
                            .file_stem()
                            .ok_or(Error::Path("non-ascii path".into()))?
                            .to_str()
                            .ok_or(Error::Path("non-ascii path".into()))?
                            .to_string();
                        let contents = std::fs::read_to_string(path)?;
                        self.responses
                            .insert(name, serde_yaml::from_str(&contents)?);
                    }
                }
            }
        }
        Ok(())
    }

    pub fn merge(&mut self, other: Config) {
        self.contexts.extend(other.contexts);
        self.requests.extend(other.requests);
        self.responses.extend(other.responses);
        self.tests.extend(other.tests);
    }

    pub fn merge_contexts(&self, names: &[String]) -> Result<HashMap<String, String>> {
        let mut context: HashMap<String, String> = HashMap::new();
        for n in names {
            match self.contexts.get(n) {
                Some(c) => {
                    context.extend(c.iter().map(|(k, v)| (k.clone(), v.clone())));
                }
                None => {
                    return Err(Error::ContextNotFound(n.clone()));
                }
            };
        }
        Ok(context)
    }
}

impl std::fmt::Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let c = serde_yaml::to_string(&self).unwrap();
        write!(f, "{}", c)
    }
}

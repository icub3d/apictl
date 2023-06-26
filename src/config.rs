use std::collections::HashMap;
use std::path::PathBuf;

use crate::{Context, Request, Response};

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use thiserror::Error;
use walkdir::WalkDir;

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

#[derive(Debug, Deserialize, Serialize)]
pub struct Config<C: Context> {
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub contexts: HashMap<String, C>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub requests: HashMap<String, Request>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub responses: HashMap<String, Response>,
}

type Result<T> = std::result::Result<T, Error>;

impl<C: Context> Default for Config<C> {
    fn default() -> Self {
        Self {
            contexts: HashMap::default(),
            requests: HashMap::default(),
            responses: HashMap::default(),
        }
    }
}

impl<C: Context + DeserializeOwned + Default> Config<C> {
    pub fn new(path: &str) -> Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        Ok(serde_yaml::from_str(&contents)?)
    }

    pub fn new_from_dir(path: &PathBuf) -> Result<Self> {
        let mut cfg: Config<C> = Config::default();
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

    pub fn merge(&mut self, other: Config<C>) {
        self.contexts.extend(other.contexts);
        self.requests.extend(other.requests);
        self.responses.extend(other.responses);
    }

    pub fn merge_contexts(&self, names: &[String]) -> Result<C> {
        let mut context: C = C::default();
        for n in names {
            context.merge(
                self.contexts
                    .get(n)
                    .ok_or(Error::ContextNotFound(n.clone()))?,
            );
        }
        Ok(context)
    }
}

impl<C: Context + Serialize> std::fmt::Display for Config<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let c = serde_yaml::to_string(&self).unwrap();
        write!(f, "{}", c)
    }
}

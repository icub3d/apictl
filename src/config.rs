use crate::context::Contexts;
use crate::request::Requests;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("yaml parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub contexts: Contexts,
    #[serde(default)]
    pub requests: Requests,
}

type Result<T> = std::result::Result<T, Error>;

impl Default for Config {
    fn default() -> Self {
        Self {
            contexts: Contexts::new(),
            requests: Requests::new(),
        }
    }
}

impl Config {
    pub fn new(path: &str) -> Result<Config> {
        let contents = std::fs::read_to_string(path)?;
        Ok(serde_yaml::from_str(&contents)?)
    }

    pub fn merge(&mut self, other: Config) {
        self.contexts.merge(other.contexts);
        self.requests.merge(other.requests);
    }
}

impl std::fmt::Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let c = serde_yaml::to_string(&self).unwrap();
        write!(f, "{}", c)
    }
}

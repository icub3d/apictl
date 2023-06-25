use std::collections::HashMap;

use crate::{Context, Request, Response};

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("yaml parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),
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

    pub fn merge(&mut self, other: Config<C>) {
        self.contexts.extend(other.contexts);
        self.requests.extend(other.requests);
        self.responses.extend(other.responses);
    }
}

impl<C: Context + Serialize> std::fmt::Display for Config<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let c = serde_yaml::to_string(&self).unwrap();
        write!(f, "{}", c)
    }
}

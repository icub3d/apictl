use std::collections::HashMap;
use std::path::PathBuf;

use crate::{Request, Response};

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
    }

    pub fn get_merge_contexts(&self, names: &[String]) -> Result<HashMap<String, String>> {
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

    pub fn apply(&self, cxt: &HashMap<String, String>, s: &str) -> String {
        let mut output = String::new();
        let mut last = 0;

        for capture in crate::VARIABLE.captures_iter(s) {
            let r = capture.get(0).unwrap().range();
            let name = capture.get(1).unwrap().as_str();
            output.push_str(&s[last..r.start]);
            let replacement = match name.starts_with("response.") {
                true => match self.find_response_data(&name[9..]) {
                    Some(v) => v,
                    None => "".to_string(),
                },
                false => match cxt.get(name) {
                    Some(v) => v.clone(),
                    None => "".to_string(),
                },
            };

            output.push_str(&replacement);

            last = r.end;
        }

        output.push_str(&s[last..]);
        output
    }

    fn find_response_data(&self, name: &str) -> Option<String> {
        use serde_json::value::Index;

        let tokens = name.split('.').collect::<Vec<_>>();
        if tokens.len() < 2 {
            return None;
        }
        let response = self.responses.get(tokens[0])?;
        let mut cur: serde_json::Value = serde_json::from_str(&response.body).ok()?;
        for token in tokens[1..tokens.len()].iter() {
            let t: Box<dyn Index> = match token.parse::<usize>() {
                Ok(v) => Box::new(v),
                Err(_) => Box::new(token),
            };
            cur = match cur.get(t.as_ref()) {
                Some(v) => v.clone(),
                None => return None,
            }
        }
        Some(
            cur.to_string()
                .trim_start_matches('"')
                .trim_end_matches('"')
                .to_string(),
        )
    }
}

impl std::fmt::Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let c = serde_yaml::to_string(&self).unwrap();
        write!(f, "{}", c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply() {
        let mut cxt = HashMap::new();
        cxt.insert("name".to_string(), "World".to_string());
        cxt.insert("age".to_string(), "4.543 Billion".to_string());

        let mut cfg = Config::default();
        cfg.responses.insert(
            "hello".to_string(),
            Response {
                status_code: 200,
                headers: HashMap::new(),
                body: "{ \"name\": \"Galaxy\", \"age\": \"13.61 Billion\" }".to_string(),
            },
        );

        let s = cfg.apply(&cxt, "Hello, ${name}! You are ${age} years old. My name is ${response.hello.name}. I am ${response.hello.age} years old.${response.hello.some.bad.one}${response.}");
        assert_eq!(
            s,
            "Hello, World! You are 4.543 Billion years old. My name is Galaxy. I am 13.61 Billion years old."
        );
    }
}

use std::{collections::HashMap, path::PathBuf};

use crate::{Config, Context, List};

use serde::{Deserialize, Serialize};
use thiserror::Error;

impl List for HashMap<String, Response> {
    fn headers(&self) -> Vec<String> {
        vec!["Name".into(), "Status".into()]
    }

    fn values(&self) -> Vec<Vec<String>> {
        self.iter()
            .map(|(n, r)| vec![n.clone(), r.status_code.to_string()])
            .collect()
    }
}

#[derive(Error, Debug)]
pub enum RequestError {
    #[error("http error: {0}")]
    Http(reqwest::Error),

    #[error("non-ascii header: {0}")]
    NonAsciiHeader(reqwest::header::ToStrError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("yaml parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),
}

pub type Result<T> = std::result::Result<T, RequestError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl Response {
    pub async fn from(response: reqwest::Response) -> Result<Self> {
        let status_code = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .map(|(k, v)| {
                Ok((
                    k.to_string(),
                    v.to_str()
                        .map_err(RequestError::NonAsciiHeader)?
                        .to_string(),
                ))
            })
            .collect::<Result<_>>()?;
        let body = response.text().await.map_err(RequestError::Http)?;
        Ok(Self {
            status_code,
            headers,
            body,
        })
    }

    pub fn save<C: Context + Serialize>(&self, name: &str, path: &PathBuf) -> Result<()> {
        let mut config: Config<C> = Config::default();
        config.responses.insert(name.to_string(), self.clone());
        std::fs::write(path, serde_yaml::to_string(&config)?).map_err(RequestError::Io)
    }
}

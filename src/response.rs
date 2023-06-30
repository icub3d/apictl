use std::{collections::HashMap, path::Path};

use crate::List;

use serde::{Deserialize, Serialize};
use thiserror::Error;

impl List for HashMap<String, Response> {
    fn headers(&self) -> Vec<String> {
        vec!["Name".into(), "Content-Type".into(), "Status".into()]
    }

    fn values(&self) -> Vec<Vec<String>> {
        self.iter()
            .map(|(n, r)| {
                vec![
                    n.clone(),
                    r.headers
                        .get("content-type")
                        .unwrap_or(&"".to_string())
                        .clone(),
                    r.status_code.to_string(),
                ]
            })
            .collect()
    }
}

#[derive(Error, Debug)]
pub enum ResponseError {
    #[error("http error: {0}")]
    Http(reqwest::Error),

    #[error("non-ascii header: {0}")]
    NonAsciiHeader(reqwest::header::ToStrError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("yaml parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),
}

pub type Result<T> = std::result::Result<T, ResponseError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub status_code: u16,
    pub version: String,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl std::fmt::Display for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut headers = self
            .headers
            .iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect::<Vec<_>>();
        headers.sort();
        write!(
            f,
            "{} {}\r\n\r\n{}\r\n\r\n{}",
            self.version,
            self.status_code,
            headers.join("\r\n"),
            self.body
        )
    }
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
                        .map_err(ResponseError::NonAsciiHeader)?
                        .to_string(),
                ))
            })
            .collect::<Result<_>>()?;
        let version = format!("{:?}", &response.version());
        let body = response.text().await.map_err(ResponseError::Http)?;
        Ok(Self {
            version,
            status_code,
            headers,
            body,
        })
    }

    pub fn save(&self, cache_dir: &Path, name: &str) -> Result<()> {
        let path = cache_dir.join(format!("{}.yaml", name));
        std::fs::write(path, serde_yaml::to_string(&self)?).map_err(ResponseError::Io)
    }
}

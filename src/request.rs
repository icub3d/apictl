use std::collections::HashMap;

use crate::{Config, List, Response, ResponseError};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Implement List for Requests.
impl List for HashMap<String, Request> {
    fn headers(&self) -> Vec<String> {
        vec![
            "Name".into(),
            "Method".into(),
            "URL".into(),
            "Description".into(),
        ]
    }

    fn values(&self) -> Vec<Vec<String>> {
        self.iter()
            .map(|(n, r)| {
                vec![
                    n.clone(),
                    r.method.clone(),
                    r.url.clone(),
                    r.description.clone(),
                ]
            })
            .collect()
    }
}

/// RequestError is the error type for requests.
#[derive(Error, Debug)]
pub enum RequestError {
    #[error("http error: {0}")]
    Http(reqwest::Error),

    #[error("io error: {0}")]
    Io(std::io::Error),

    #[error("response parse error: {0}")]
    Parse(ResponseError),

    #[error("unsupported method: {0}")]
    UnsupportedMethod(String),
}

/// Result is the result type for requests.
type Result<T> = std::result::Result<T, RequestError>;

/// Requests from the configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Request {
    pub description: String,
    pub tags: Vec<String>,
    pub url: String,
    #[serde(default = "default_method")]
    pub method: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub query_parameters: HashMap<String, String>,
    #[serde(default)]
    pub body: Body,
}

fn default_method() -> String {
    "GET".to_string()
}

impl Request {
    /// Apply the configuration and context to the request. All parts
    /// of the request are replaced with the response values and
    /// contexts.
    pub fn apply(&mut self, cfg: &Config, cxt: &HashMap<String, String>) {
        self.url = cfg.apply(cxt, &self.url);
        self.method = cfg.apply(cxt, &self.method);
        for value in self.headers.values_mut() {
            *value = cfg.apply(cxt, value);
        }
        for value in self.query_parameters.values_mut() {
            *value = cfg.apply(cxt, value);
        }
        match &mut self.body {
            Body::None => {}
            Body::Form { data } => {
                for value in data.values_mut() {
                    *value = cfg.apply(cxt, value);
                }
            }
            Body::Raw { from } => match from {
                RawBody::File { path } => {
                    *path = cfg.apply(cxt, path);
                }
                RawBody::Text { data } => {
                    *data = cfg.apply(cxt, data);
                }
            },
            Body::MultiPart { data } => {
                for value in data.values_mut() {
                    match value {
                        MultiPartField::Text { data } => {
                            *data = cfg.apply(cxt, data);
                        }
                        MultiPartField::File { path } => {
                            *path = cfg.apply(cxt, path);
                        }
                    }
                }
            }
        }
    }

    /// Perform the request and return it's response.
    pub async fn request(&self) -> Result<Response> {
        use reqwest::Client;

        let mut builder = match self.method.as_str() {
            "GET" => Client::new().get(&self.url),
            "POST" => Client::new().post(&self.url),
            "PUT" => Client::new().put(&self.url),
            "DELETE" => Client::new().delete(&self.url),
            _ => return Err(RequestError::UnsupportedMethod(self.method.clone())),
        };

        for (key, value) in self.headers.iter() {
            builder = builder.header(key, value);
        }

        builder = builder.query(&self.query_parameters);

        match &self.body {
            Body::None => {}
            Body::Form { data } => {
                builder = builder.form(data);
            }
            Body::Raw { from } => match from {
                RawBody::File { path } => {
                    builder =
                        builder.body(std::fs::read_to_string(path).map_err(RequestError::Io)?);
                }
                RawBody::Text { data } => {
                    builder = builder.body(data.clone());
                }
            },
            Body::MultiPart { data } => {
                let mut form = reqwest::multipart::Form::new();
                for (key, value) in data.iter() {
                    match value {
                        MultiPartField::Text { data } => {
                            form = form.text(key.clone(), data.clone());
                        }
                        MultiPartField::File { path } => {
                            let mut part = reqwest::multipart::Part::stream(
                                tokio::fs::File::open(path)
                                    .await
                                    .map_err(RequestError::Io)?,
                            );
                            part = part.file_name(path.clone());
                            form = form.part(key.clone(), part);
                        }
                    }
                }
                builder = builder.multipart(form);
            }
        }

        Response::from(builder.send().await.map_err(RequestError::Http)?)
            .await
            .map_err(RequestError::Parse)
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Body {
    #[default]
    None,
    Form {
        data: HashMap<String, String>,
    },
    Raw {
        from: RawBody,
    },
    MultiPart {
        data: HashMap<String, MultiPartField>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum RawBody {
    File { path: String },
    Text { data: String },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum MultiPartField {
    File { path: String },
    Text { data: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize() {
        let request = r#"
tags: [post, form]
description: post using key/value pairs
url: https://api.example.com/endpoint1
method: POST
headers:
  Authorization: Bearer your-token
body:
  type: form
  data:
    key1: value1
    key2: value2
"#;

        let request: Request = serde_yaml::from_str(request).unwrap();

        assert_eq!(request.description, "post using key/value pairs");
        assert_eq!(request.tags, vec!["post", "form"]);
        assert_eq!(request.url, "https://api.example.com/endpoint1");
        assert_eq!(request.method, "POST");
        assert_eq!(request.headers.len(), 1);
        assert_eq!(
            request.body,
            Body::Form {
                data: vec![
                    ("key1".to_string(), "value1".to_string()),
                    ("key2".to_string(), "value2".to_string()),
                ]
                .into_iter()
                .collect()
            }
        );
    }

    #[test]
    fn apply() {
        let request = r#"
tags: [post, form]
description: post using key/value pairs
url: "${base_url}/endpoint1"
method: POST
headers:
  Authorization: "Bearer ${token}"
body:
  type: form
  data:
    key1: "${value1}"
    key2: value2
"#;

        let mut request: Request = serde_yaml::from_str(request).unwrap();
        let mut cxt = HashMap::new();
        cxt.extend(vec![
            (
                "base_url".to_string(),
                "https://api.example.com".to_string(),
            ),
            ("token".to_string(), "your-token".to_string()),
            ("value1".to_string(), "value1".to_string()),
        ]);

        let cfg = Config::default();

        request.apply(&cfg, &cxt);

        assert_eq!(request.description, "post using key/value pairs");
        assert_eq!(request.tags, vec!["post", "form"]);
        assert_eq!(request.url, "https://api.example.com/endpoint1");
        assert_eq!(request.method, "POST");
        assert_eq!(request.headers.len(), 1);
        assert_eq!(
            request.body,
            Body::Form {
                data: vec![
                    ("key1".to_string(), "value1".to_string()),
                    ("key2".to_string(), "value2".to_string()),
                ]
                .into_iter()
                .collect()
            }
        );
    }
}

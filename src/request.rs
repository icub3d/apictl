use std::collections::HashMap;

use crate::context::Context;
use crate::List;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Requests {
    #[serde(flatten)]
    pub requests: HashMap<String, Request>,
}

impl Default for Requests {
    fn default() -> Self {
        Self::new()
    }
}

impl Requests {
    pub fn new() -> Self {
        Self {
            requests: HashMap::new(),
        }
    }

    pub fn merge(&mut self, other: Requests) {
        for (k, v) in other.requests {
            self.requests.insert(k, v);
        }
    }
}

impl IntoIterator for Requests {
    type Item = Vec<String>;
    type IntoIter = std::iter::Map<
        std::collections::hash_map::IntoIter<String, Request>,
        fn((String, Request)) -> Vec<String>,
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.requests.into_iter().map(|(name, request)| {
            vec![
                name.clone(),
                request.method.clone(),
                request.url.clone(),
                request.description.clone(),
            ]
        })
    }
}

impl List for Requests {
    fn headers(&self) -> Vec<String> {
        vec![
            "Name".into(),
            "Method".into(),
            "URL".into(),
            "Description".into(),
        ]
    }
}

#[derive(Debug, Serialize, Deserialize)]
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
    pub payload: Payload,
}

fn default_method() -> String {
    "GET".to_string()
}

impl Request {
    pub fn apply(&mut self, context: &Context) {
        self.description = context.apply(&self.description);
        self.url = context.apply(&self.url);
        self.method = context.apply(&self.method);
        for (_, value) in &mut self.headers {
            *value = context.apply(value);
        }
        for (_, value) in &mut self.query_parameters {
            *value = context.apply(value);
        }
        match &mut self.payload {
            Payload::None => {}
            Payload::Form { data } => {
                for (_, value) in data {
                    *value = context.apply(value);
                }
            }
            Payload::Raw { body } => match body {
                RawPayload::File { path } => {
                    *path = context.apply(path);
                }
                RawPayload::Raw { data } => {
                    *data = context.apply(data);
                }
            },
            Payload::MultiPart { data } => {
                for (_, value) in data {
                    match value {
                        MultiPartField::Text { data } => {
                            *data = context.apply(data);
                        }
                        MultiPartField::File { path } => {
                            *path = context.apply(path);
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Payload {
    None,
    Form {
        data: HashMap<String, String>,
    },
    Raw {
        body: RawPayload,
    },
    MultiPart {
        data: HashMap<String, MultiPartField>,
    },
}

impl Default for Payload {
    fn default() -> Self {
        Payload::None
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum RawPayload {
    File { path: String },
    Raw { data: String },
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum MultiPartField {
    Text { data: String },
    File { path: String },
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
payload:
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
            request.payload,
            Payload::Form {
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
url: "{{base_url}}/endpoint1"
method: POST
headers:
  Authorization: "Bearer {{token}}"
payload:
  type: form
  data:
    key1: "{{value1}}"
    key2: value2
"#;

        let mut request: Request = serde_yaml::from_str(request).unwrap();
        let context: HashMap<String, String> = vec![
            (
                "base_url".to_string(),
                "https://api.example.com".to_string(),
            ),
            ("token".to_string(), "your-token".to_string()),
            ("value1".to_string(), "value1".to_string()),
        ]
        .into_iter()
        .collect();
        dbg!(context.clone());
        request.apply(&context);

        assert_eq!(request.description, "post using key/value pairs");
        assert_eq!(request.tags, vec!["post", "form"]);
        assert_eq!(request.url, "https://api.example.com/endpoint1");
        assert_eq!(request.method, "POST");
        assert_eq!(request.headers.len(), 1);
        assert_eq!(
            request.payload,
            Payload::Form {
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
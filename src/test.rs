use std::collections::HashMap;

use crate::{Applicator, Config, List, Response};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Implement List for Requests.
impl List for HashMap<String, Test> {
    fn headers(&self) -> Vec<String> {
        vec!["Name".into(), "Steps".into(), "Description".into()]
    }

    fn values(&self) -> Vec<Vec<String>> {
        self.iter()
            .map(|(n, t)| vec![n.clone(), t.steps.len().to_string(), t.description.clone()])
            .collect()
    }
}

/// TestError is the error type for tests.
#[derive(Error, Debug)]
pub enum TestError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("request not found: {0}")]
    RequestNotFound(String),

    #[error("request error: {0}")]
    RequestError(#[from] crate::RequestError),

    #[error("yaml error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("assert failed: {0}")]
    AssertError(String),

    #[error("regex error: {0}")]
    RegexError(#[from] regex::Error),
}

/// Result is the result type for tests.
pub type Result<T> = std::result::Result<T, TestError>;

/// TestState is the current state of a test.
#[derive(Default, Debug)]
pub enum TestState {
    /// NotRun indicates that the test has not been run.
    #[default]
    NotRun,

    // TODO: (?) we could potentially do running later.
    /// Running indicates that the test is currently running.
    Running,

    /// Passed indicates that the test has passed.
    Passed,

    /// Failed indicates that the test has failed.
    Failed(String),
}

impl std::fmt::Display for TestState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestState::NotRun => write!(f, "⏸"),
            TestState::Running => write!(f, "🏃"),
            TestState::Passed => write!(f, "✅"),
            TestState::Failed(_) => write!(f, "❌"),
        }
    }
}

#[derive(Debug)]
pub struct TestResults {
    pub name: String,
    pub state: TestState,
    pub children: Vec<TestResults>,
}

impl TestResults {
    pub fn new(name: String) -> Self {
        Self {
            name,
            state: TestState::NotRun,
            children: Vec::new(),
        }
    }

    pub fn create_child(&mut self, name: String, state: TestState) {
        self.children.push(TestResults {
            name,
            state,
            children: Vec::new(),
        });
    }

    pub fn add_child(&mut self, child: TestResults) {
        self.children.push(child);
    }

    pub fn complete(&mut self) {
        self.state = TestState::Passed;
        for child in &self.children {
            if let TestState::Failed(_) = child.state {
                self.state = TestState::Failed("dependent test failed".into());
                break;
            }
        }
    }

    pub fn print(&self, prefix: &str) {
        println!("{}{} {}", prefix, self.state, self.name);
        for child in &self.children {
            child.print(&format!("{}  ", prefix));
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Test {
    description: String,
    steps: Vec<Step>,
}

impl Test {
    pub async fn execute(
        &self,
        name: String,
        cfg: &Config,
        context: &HashMap<String, String>,
    ) -> Result<TestResults> {
        let mut results = TestResults::new(name);
        let mut app = Applicator::new(context.clone(), cfg.responses.clone());
        for step in &self.steps {
            let mut request = match cfg.requests.get(&step.request) {
                Some(r) => r.clone(),
                None => {
                    // TODO (?) return a test result here as well?
                    return Err(TestError::RequestNotFound(step.request.clone()));
                }
            };
            request.apply(&app);

            let resp: Response = request.request().await.map_err(TestError::RequestError)?;
            // Save the response incase it is used by a later request.
            app.add_response(step.request.clone(), resp.clone());

            let mut step_result = TestResults::new(step.name.clone());
            for assert in &step.asserts {
                step_result.create_child(
                    format!("{}", assert),
                    match assert.execute(&resp) {
                        Ok(_) => TestState::Passed,
                        Err(e) => TestState::Failed(e.to_string()),
                    },
                );
            }
            step_result.complete();
            results.add_child(step_result);
        }
        results.complete();
        Ok(results)
    }
}

impl std::fmt::Display for Test {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut steps = self
            .steps
            .iter()
            .map(|s| format!("{}", s))
            .collect::<Vec<_>>();
        steps.sort();
        write!(
            f,
            "  description: {}\n  steps:\n{}",
            self.description,
            steps.join("\n")
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    name: String,
    request: String,
    asserts: Vec<Assert>,
}

impl std::fmt::Display for Step {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut asserts = self
            .asserts
            .iter()
            .map(|a| format!("    {}", a))
            .collect::<Vec<_>>();
        asserts.sort();
        write!(
            f,
            "   {} ({})\n    asserts:\n  {}",
            self.name,
            self.request,
            asserts.join("\n  ")
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Assert {
    StatusCode { value: u16 },
    HeaderContains { key: String, value: String },
    HeaderEquals { key: String, value: String },
    Contains { key: String, value: String },
    Equals { key: String, value: String },
    NotEquals { key: String, value: String },
    HasPrefix { key: String, value: String },
    HasSuffix { key: String, value: String },
    Regex { key: String, value: String },
}

impl Assert {
    pub fn execute(&self, response: &Response) -> Result<()> {
        match self {
            Assert::StatusCode { value } => {
                if response.status_code != *value {
                    return Err(TestError::AssertError(format!(
                        "got status code {}, want {}",
                        response.status_code, value
                    )));
                }
            }
            Assert::HeaderContains { key, value } => {
                let header = response
                    .headers
                    .get(key)
                    .ok_or_else(|| TestError::AssertError(format!("header not found: {}", key)))?;
                if !header.contains(value) {
                    return Err(TestError::AssertError(format!(
                        "header '{}' got '{}', does not contain '{}'",
                        key, header, value
                    )));
                }
            }
            Assert::HeaderEquals { key, value } => {
                let header = response
                    .headers
                    .get(key)
                    .ok_or_else(|| TestError::AssertError(format!("header not found: {}", key)))?;
                if header != value {
                    return Err(TestError::AssertError(format!(
                        "header '{}' got '{}', want '{}'",
                        key, header, value
                    )));
                }
            }
            Assert::Contains { key, value } => {
                let result = response
                    .find_path_in_body(key)
                    .ok_or(TestError::AssertError(format!(
                        "key '{}' not found in request",
                        key
                    )))?;
                if !result.contains(value) {
                    return Err(TestError::AssertError(format!(
                        "body '{}' got '{}', does not contain '{}'",
                        key.clone(),
                        result,
                        value.clone(),
                    )));
                }
            }
            Assert::Equals { key, value } => {
                let result = response
                    .find_path_in_body(key)
                    .ok_or(TestError::AssertError(format!(
                        "key '{}' not found in request",
                        key
                    )))?;
                if result != *value {
                    return Err(TestError::AssertError(format!(
                        "body '{}' got '{}', want '{}'",
                        key.clone(),
                        result,
                        value.clone(),
                    )));
                }
            }
            Assert::NotEquals { key, value } => {
                let result = response
                    .find_path_in_body(key)
                    .ok_or(TestError::AssertError(format!(
                        "key '{}' not found in request",
                        key
                    )))?;
                if result == *value {
                    return Err(TestError::AssertError(format!(
                        "body '{}' got '{}', did not want '{}'",
                        key.clone(),
                        result,
                        value.clone(),
                    )));
                }
            }
            Assert::HasPrefix { key, value } => {
                let result = response
                    .find_path_in_body(key)
                    .ok_or(TestError::AssertError(format!(
                        "key '{}' not found in request",
                        key
                    )))?;
                if !result.starts_with(value) {
                    return Err(TestError::AssertError(format!(
                        "body '{}' got '{}', does not have prefix '{}'",
                        key.clone(),
                        result,
                        value.clone(),
                    )));
                }
            }
            Assert::HasSuffix { key, value } => {
                let result = response
                    .find_path_in_body(key)
                    .ok_or(TestError::AssertError(format!(
                        "key '{}' not found in request",
                        key
                    )))?;
                if !result.ends_with(value) {
                    return Err(TestError::AssertError(format!(
                        "body '{}' got '{}', does not have suffix '{}'",
                        key.clone(),
                        result,
                        value.clone(),
                    )));
                }
            }
            Assert::Regex { key, value } => {
                let result = response
                    .find_path_in_body(key)
                    .ok_or(TestError::AssertError(format!(
                        "key '{}' not found in request",
                        key
                    )))?;
                let re = regex::Regex::new(value).map_err(TestError::RegexError)?;
                if !re.is_match(&result) {
                    return Err(TestError::AssertError(format!(
                        "body '{}' got '{}', does not match regex '{}'",
                        key.clone(),
                        result,
                        value.clone(),
                    )));
                }
            }
        }
        Ok(())
    }
}

impl std::fmt::Display for Assert {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Assert::StatusCode { value } => write!(f, "status_code == {}", value),
            Assert::HeaderContains { key, value } => {
                write!(f, "header_contains({}, {})", key, value)
            }
            Assert::HeaderEquals { key, value } => write!(f, "header_equals({}, {})", key, value),
            Assert::Contains { key, value } => write!(f, "contains({}, {})", key, value),
            Assert::Equals { key, value } => write!(f, "equals({}, {})", key, value),
            Assert::NotEquals { key, value } => write!(f, "not_equals({}, {})", key, value),
            Assert::HasPrefix { key, value } => write!(f, "has_prefix({}, {})", key, value),
            Assert::HasSuffix { key, value } => write!(f, "has_suffix({}, {})", key, value),
            Assert::Regex { key, value } => write!(f, "regex({}, {})", key, value),
        }
    }
}

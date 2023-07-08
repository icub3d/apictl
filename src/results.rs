use std::io::{Stdout, Write};
use std::time::{Duration, Instant};

use crossterm::{cursor, terminal, ExecutableCommand};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// ResultsError is the error type for results.
#[derive(Error, Debug)]
pub enum ResultsError {
    #[error("terminal error: {0}")]
    TerminalError(std::io::Error),
}

/// Result is the result type for tests.
pub type Result<T> = std::result::Result<T, ResultsError>;

/// State is the current state of a result.
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub enum State {
    /// NotRun indicates that the result has not been run.
    #[default]
    NotRun,

    // TODO: (?) we could potentially do running later.
    /// Running indicates that the result is currently running.
    Running,

    /// Passed indicates that the result has passed.
    Passed,

    /// Failed indicates that the result has failed.
    Failed(String),
}

impl std::fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            State::NotRun => write!(f, "⏸"),
            State::Running => write!(f, "🏃"),
            State::Passed => write!(f, "✅"),
            State::Failed(_) => write!(f, "❌"),
        }
    }
}

#[derive(Debug)]
pub struct Results {
    pub name: String,
    pub state: State,
    pub duration: Duration,
    pub children: Vec<Results>,
}

impl Results {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            state: State::NotRun,
            duration: Duration::default(),
            children: Vec::new(),
        }
    }

    pub fn add(&mut self, name: &str) {
        self.children.push(Self::new(name));
    }

    pub fn add_results(&mut self, results: Results) {
        self.children.push(results);
    }

    pub fn from_test(name: &str, test: &crate::Test) -> Self {
        Self {
            name: name.to_string(),
            state: State::NotRun,
            duration: Duration::default(),
            children: test
                .steps
                .iter()
                .map(|s| Self {
                    name: s.name.clone(),
                    state: State::NotRun,
                    duration: Duration::default(),
                    children: s
                        .asserts
                        .iter()
                        .map(|a| Self {
                            name: format!("{}", a),
                            state: State::NotRun,
                            duration: Duration::default(),
                            children: Vec::new(),
                        })
                        .collect(),
                })
                .collect(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    pub fn len(&self) -> usize {
        let mut len = 1;
        for child in &self.children {
            len += child.len();
        }
        len
    }

    pub fn update(&mut self, names: &[String], state: State, start: Instant) {
        if names.len() == 1 && self.name == names[0] {
            self.duration = start.elapsed();
            self.state = state;
        } else if !names.is_empty() && self.name == names[0] {
            let child = self
                .children
                .iter_mut()
                .find(|c| c.name == names[1])
                .unwrap();
            child.update(&names[1..], state, start);
        }
    }

    pub fn print(&self, s: &mut Stdout, prefix: &str) -> Result<()> {
        writeln!(
            s,
            "{}{} ({:?}) {}",
            prefix, self.state, self.duration, self.name
        )
        .map_err(ResultsError::TerminalError)?;
        for child in &self.children {
            child.print(s, &format!("{}  ", prefix))?;
        }
        Ok(())
    }

    pub fn output(&self, s: &mut Stdout, prefix: &str) -> Result<()> {
        self.clear(s)?;
        writeln!(
            s,
            "{}{} ({:?}) {}",
            prefix, self.state, self.duration, self.name
        )
        .map_err(ResultsError::TerminalError)?;
        for child in &self.children {
            child.print(s, &format!("{}  ", prefix))?;
        }
        Ok(())
    }

    pub fn clear(&self, s: &mut Stdout) -> Result<()> {
        s.execute(cursor::MoveUp(self.len() as u16))
            .map_err(ResultsError::TerminalError)?;
        s.execute(terminal::Clear(terminal::ClearType::FromCursorDown))
            .map_err(ResultsError::TerminalError)?;
        Ok(())
    }
}

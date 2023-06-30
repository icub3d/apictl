/// Output is used to help output commands in a variety of formats.
use std::collections::HashMap;

use prettytable::{Cell, Row, Table};
use serde::Serialize;
use thiserror::Error;

/// OutputFormat is the format to output the data in.
#[derive(Clone)]
pub enum OutputFormat {
    /// uses prettytable
    Table,
    /// tab delimited
    TSV,
    /// yaml
    Yaml,
}

/// Errors that can occur when outputting data.
#[derive(Error, Debug)]
pub enum OutputError {
    #[error("yaml parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("format error: {0}")]
    Format(String),
}

/// Result is a convenience type for output results.
pub type Result<T> = std::result::Result<T, OutputError>;

impl std::str::FromStr for OutputFormat {
    type Err = OutputError;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "table" => Ok(OutputFormat::Table),
            "tsv" => Ok(OutputFormat::TSV),
            "yaml" => Ok(OutputFormat::Yaml),
            _ => Err(OutputError::Format(format!("unknown format: {}", s))),
        }
    }
}

/// List is a trait for types that can be output.
pub trait List: Serialize {
    /// Returns the headers (fields) for the output.
    fn headers(&self) -> Vec<String>;

    /// Returns the values for the output.
    fn values(&self) -> Vec<Vec<String>>;

    /// Outputs the data in the given format.
    fn output(&self, format: OutputFormat) -> Result<()> {
        match format {
            OutputFormat::Yaml => {
                println!("{}", serde_yaml::to_string(&self)?);
            }
            OutputFormat::TSV => {
                for l in self.values() {
                    println!("{}", l.join("\t"));
                }
            }
            OutputFormat::Table => {
                let mut table = Table::new();
                let mut header = Row::empty();
                for h in self.headers() {
                    header.add_cell(Cell::new(&h).style_spec("b"));
                }
                table.add_row(header);
                for l in self.values() {
                    let mut row = Row::empty();
                    for c in l {
                        row.add_cell(Cell::new(&c));
                    }
                    table.add_row(row);
                }
                table.printstd();
            }
        };

        Ok(())
    }
}

/// This will implement List for Contexts.
impl List for HashMap<String, HashMap<String, String>> {
    fn headers(&self) -> Vec<String> {
        vec!["Name".into()]
    }

    fn values(&self) -> Vec<Vec<String>> {
        self.iter().map(|(n, _)| vec![n.clone()]).collect()
    }
}

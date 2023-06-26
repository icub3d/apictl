use prettytable::{Cell, Row, Table};
use serde::Serialize;
use thiserror::Error;

#[derive(Clone)]
pub enum OutputFormat {
    Table,
    TSV,
    Yaml,
}

#[derive(Error, Debug)]
pub enum OutputError {
    #[error("yaml parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("format error: {0}")]
    Format(String),
}

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

pub trait List: Serialize {
    fn headers(&self) -> Vec<String>;
    fn values(&self) -> Vec<Vec<String>>;

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

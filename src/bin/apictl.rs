use std::fs;
use std::path::PathBuf;

use apictl::{config::Config, context::Context, List};

use anyhow::Result;
use clap::{Parser, Subcommand};
use prettytable::{Cell, Row, Table};

#[derive(Parser)]
#[command(name = "apictl")]
#[command(author = "Joshua Marsh (icub3d) <joshua.marshian@gmail.com")]
#[command(about = "A command line interface for making API calls.")]
#[command(version = "0.1")]
#[command(long_about = None)]
struct Args {
    /// The folder containing the configuration files.
    #[arg(short, long, value_name = "CONFIG", default_value = ".apictl")]
    config: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Manage requests.
    #[command(subcommand)]
    Requests(Requests),

    /// Manage contexts.
    #[command(subcommand)]
    Contexts(Contexts),
}

#[derive(Subcommand)]
enum Requests {
    /// List all the requests.
    List {
        /// The format in which to display the requests.
        #[arg(short, long, value_name = "OUTPUT", default_value = "table")]
        output: OutputFormat,
    },

    /// Run the given requests.
    Run {
        /// The contexts to use.
        #[arg(short, long, value_name = "CONTEXT")]
        contexts: Vec<String>,

        /// The requests to run.
        requests: Vec<String>,
    },
}

#[derive(Subcommand)]
enum Contexts {
    /// List all the contexts.
    List {
        /// The format in which to output the requests.
        #[arg(short, long, value_name = "OUTPUT", default_value = "tsv")]
        output: OutputFormat,
    },
}

#[derive(Clone)]
enum OutputFormat {
    Table,
    TSV,
    Yaml,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "table" => Ok(OutputFormat::Table),
            "tsv" => Ok(OutputFormat::TSV),
            "yaml" => Ok(OutputFormat::Yaml),
            _ => Err(format!("Unknown format: {}", s)),
        }
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Parse configuration files.
    let mut cfg = Config::default();
    if let Ok(entries) = fs::read_dir(args.config) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(extension) = path.extension() {
                if extension == "yaml" || extension == "yml" {
                    cfg.merge(Config::new(path.to_str().unwrap())?);
                }
            }
        }
    } else {
        println!("Failed to read the folder.");
    }

    // Execute the command.
    match args.command {
        Command::Requests(requests) => match requests {
            Requests::List { output } => {
                display(output, cfg.requests)?;
            }
            Requests::Run { contexts, requests } => {
                let mut context = Context::new();
                for c in &contexts {
                    context.merge(cfg.contexts.contexts.get(c).unwrap());
                }
                println!("{:?}", context);
                for r in requests {
                    let request = cfg.requests.requests.get_mut(&r).unwrap();
                    request.apply(&context);
                    println!("{:?}", request);
                }
            }
        },
        Command::Contexts(contexts) => match contexts {
            Contexts::List { output } => {
                display(output, cfg.contexts)?;
            }
        },
    }

    Ok(())
}

fn display<L: List>(format: OutputFormat, list: L) -> Result<()> {
    match format {
        OutputFormat::Yaml => {
            println!("{}", serde_yaml::to_string(&list)?);
        }
        OutputFormat::TSV => {
            for l in list {
                println!("{}", l.join("\t"));
            }
        }
        OutputFormat::Table => {
            let mut table = Table::new();
            let mut header = Row::empty();
            for h in list.headers() {
                header.add_cell(Cell::new(&h).style_spec("b"));
            }
            table.add_row(header);
            for l in list {
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

use std::collections::HashMap;
use std::path::PathBuf;

use apictl::{Config, List, OutputFormat, Response};

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "apictl")]
#[command(author = "Joshua Marsh (icub3d) <joshua.marshian@gmail.com")]
#[command(about = "A command line interface for making API calls.")]
#[command(version = "0.1")]
#[command(long_about = None)]
struct Args {
    /// The folder containing the configuration and cache files.
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

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Parse configuration files.
    let mut cfg: Config<HashMap<String, String>> = Config::new_from_dir(&args.config)?;

    // Execute the command.
    match args.command {
        Command::Requests(requests) => match requests {
            Requests::List { output } => {
                cfg.requests.output(output)?;
            }
            Requests::Run { contexts, requests } => {
                let context = cfg.merge_contexts(&contexts)?;
                for r in requests {
                    let request = cfg.requests.get_mut(&r).unwrap();
                    request.apply(&context);
                    let result = request.request().await?;
                    let resp = Response::from(result).await?;
                    let mut path = args.config.clone();
                    path.push("cache");
                    std::fs::create_dir_all(&path)?;
                    path.push(&r);
                    path.set_extension("yaml");
                    resp.save::<HashMap<String, String>>(&r, &path)?;
                    println!("{}", resp.body);
                }
            }
        },
        Command::Contexts(contexts) => match contexts {
            Contexts::List { output } => {
                cfg.contexts.output(output)?;
            }
        },
    }

    Ok(())
}

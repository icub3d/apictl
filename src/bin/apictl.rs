use std::path::PathBuf;

use apictl::{Config, List, OutputFormat, Request};

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "apictl")]
#[command(author = "Joshua Marsh (icub3d) <joshua.marshian@gmail.com")]
#[command(about = "A command line interface for making API calls.")]
#[command(version = "0.1")]
#[command(long_about = None)]
struct Args {
    /// The file or folder containing the configuration and cache files.
    #[arg(short, long, value_name = "CONFIG", default_value = ".apictl.yaml")]
    config: PathBuf,

    /// The folder used to store responses.
    #[arg(long, value_name = "CACHE", default_value = ".apictl")]
    cache: PathBuf,

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

    /// Manage responses.
    #[command(subcommand)]
    Responses(Responses),
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

        /// Include response and header values before the body.
        #[arg(short, long)]
        verbose: bool,

        /// Only output errors.
        #[arg(short, long)]
        quiet: bool,
    },
}

#[derive(Subcommand)]
enum Contexts {
    /// List all the contexts.
    List {
        /// The format in which to output the contexts.
        #[arg(short, long, value_name = "OUTPUT", default_value = "tsv")]
        output: OutputFormat,
    },
}

#[derive(Subcommand)]
enum Responses {
    /// List all the response.
    List {
        /// The format in which to output the responses.
        #[arg(short, long, value_name = "OUTPUT", default_value = "tsv")]
        output: OutputFormat,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Make sure our cache dir exists
    let response_dir = args.cache.clone().join("responses");
    std::fs::create_dir_all(&response_dir)?;

    // Parse our config.
    let mut cfg = Config::new_from_path(&args.config)?;
    cfg.load_responses(&response_dir)?;

    // Execute the command.
    match args.command {
        Command::Responses(responses) => match responses {
            Responses::List { output } => {
                cfg.responses.output(output)?;
            }
        },
        Command::Contexts(contexts) => match contexts {
            Contexts::List { output } => {
                cfg.contexts.output(output)?;
            }
        },
        Command::Requests(requests) => match requests {
            Requests::List { output } => {
                cfg.requests.output(output)?;
            }
            Requests::Run {
                contexts,
                requests,
                verbose,
                quiet,
            } => {
                let context = cfg.get_merge_contexts(&contexts)?;
                let mut first = true;
                for r in requests {
                    if !first {
                        println!();
                    }
                    first = false;

                    // Get the request by name and apply the context.
                    let mut request: Request = match cfg.requests.get(&r) {
                        Some(r) => r.clone(),
                        None => {
                            return Err(anyhow::anyhow!("Request not found: {}", r));
                        }
                    };
                    request.apply(&cfg, &context);

                    // Make the requests.
                    let resp = request.request().await?;

                    // TODO stream to both places?

                    // We want to save the response to our cache and
                    // then print it out.
                    resp.save(&response_dir, &r)?;
                    if verbose && !quiet {
                        println!("{}", resp);
                    } else if !quiet {
                        println!("{}", resp.body);
                    }

                    // Save the response incase it is used by a later request.
                    cfg.responses.insert(r, resp);
                }
            }
        },
    }

    Ok(())
}

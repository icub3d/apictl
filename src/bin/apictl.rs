use std::collections::HashMap;
use std::io::stdout;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use apictl::{Applicator, Config, List, OutputFormat, Request, Response, Results, State};

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "apictl")]
#[command(author = "Joshua Marsh (icub3d) <joshua.marshian@gmail.com")]
#[command(
    about = "A command line interface for making API calls. See https://github.com/icub3d/apictl for additional details."
)]
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

    /// Manage tests.
    #[command(subcommand)]
    Tests(Tests),

    /// benchmark an API.
    Benchmark {
        /// The contexts to use.
        #[arg(short, long, value_name = "CONTEXT")]
        contexts: Vec<String>,

        /// The number of times to run the requests.
        #[arg(short, value_name = "NUMBER", default_value = "100")]
        number: usize,

        /// The number of concurrent requests to make.
        #[arg(short, value_name = "PARALLEL", default_value = "8")]
        parallel: usize,

        /// The requests to run.
        benchmarks: Vec<String>,
    },
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

#[derive(Subcommand)]
enum Tests {
    /// List all the tests.
    List {
        /// The format in which to display the requests.
        #[arg(short, long, value_name = "OUTPUT", default_value = "table")]
        output: OutputFormat,
    },

    /// Describe the given tests.
    Describe {
        /// The tests to describe.
        tests: Vec<String>,
    },

    /// Run the given tests.
    Run {
        /// The contexts to use.
        #[arg(short, long, value_name = "CONTEXT")]
        contexts: Vec<String>,

        /// The tests to run.
        tests: Vec<String>,
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
                let context = cfg.merge_contexts(&contexts)?;
                let mut app = Applicator::new(context, cfg.responses);

                for r in requests {
                    // Get the request by name and apply the context.
                    let mut request: Request = match cfg.requests.get(&r) {
                        Some(r) => r.clone(),
                        None => {
                            return Err(anyhow::anyhow!("Request not found: {}", r));
                        }
                    };
                    request.apply(&app);

                    // Make the requests.
                    let resp = request.request().await?;

                    // TODO: (?) stream to both places

                    // We want to save the response to our cache and
                    // then print it out.
                    resp.save(&response_dir, &r)?;
                    if verbose && !quiet {
                        println!("{}", resp);
                    } else if !quiet {
                        println!("{}", resp.body);
                    }

                    // Save the response incase it is used by a later request.
                    app.add_response(r, resp);
                }
            }
        },
        Command::Tests(tests) => match tests {
            Tests::List { output } => {
                cfg.tests.output(output)?;
            }
            Tests::Describe { tests } => {
                for t in tests {
                    if let Some(test) = cfg.tests.get(&t) {
                        println!("test: {}", t);
                        println!("{}", test);
                    } else {
                        dbg!(&cfg.tests);
                        return Err(anyhow::anyhow!("test not found: {}", t));
                    }
                }
            }
            Tests::Run { contexts, tests } => {
                let context = cfg.merge_contexts(&contexts)?;
                let mut results = Results::new("test results");
                let now = Instant::now();
                let mut stdout = stdout();
                for t in tests {
                    // Get the test by name and apply the context.
                    let test = match cfg.tests.get(&t) {
                        Some(t) => t,
                        None => {
                            return Err(anyhow::anyhow!("Test not found: {}", t));
                        }
                    };

                    test.execute(t, &cfg, &context, &mut results, &mut stdout)
                        .await?;
                    results.clear(&mut stdout)?;
                }

                results.state = State::Passed;
                results.duration = now.elapsed();
                results.output(&mut stdout, "")?;
            }
        },
        Command::Benchmark {
            contexts,
            number,
            parallel,
            benchmarks,
        } => {
            let context = cfg.merge_contexts(&contexts)?;
            let count = Arc::new(AtomicUsize::new(0));
            let status_codes = Arc::new(Mutex::new(HashMap::new()));
            let durations = Arc::new(Mutex::new(vec![]));
            let bar = Arc::new(Mutex::new(indicatif::ProgressBar::new(number as u64)));
            let mut handles = vec![];
            let total_duration = Instant::now();

            for _ in 0..parallel {
                let count = count.clone();
                let context = context.clone();
                let cfg = cfg.clone();
                let benchmarks = benchmarks.clone();
                let status_codes = status_codes.clone();
                let durations = durations.clone();
                let bar = bar.clone();
                let handle = tokio::spawn(async move {
                    loop {
                        let i = count.fetch_add(1, Ordering::SeqCst);
                        if i >= number {
                            return;
                        }
                        let mut app = Applicator::new(context.clone(), cfg.responses.clone());

                        for r in &benchmarks {
                            let now = Instant::now();
                            match run_request(&cfg, &mut app, r).await {
                                Ok(r) => {
                                    let mut status_codes = status_codes.lock().unwrap();
                                    *status_codes.entry(r.status_code).or_insert(0) += 1;
                                    let mut durations = durations.lock().unwrap();
                                    durations.push(now.elapsed());
                                }
                                Err(e) => {
                                    eprintln!("error: {}", e);
                                }
                            }
                        }
                        bar.lock().unwrap().inc(1);
                    }
                });
                handles.push(handle);
            }

            for handle in handles {
                handle.await?;
            }

            bar.lock().unwrap().finish();

            println!("status codes:");
            let status_codes = status_codes.lock().unwrap();
            for (code, count) in status_codes.iter() {
                println!("  {}: {}", code, count);
            }

            println!("statistics:");
            let total = number * benchmarks.len();
            println!("  total requests:     {}", total);
            println!("  total duration:     {:?}", total_duration.elapsed());
            let mean = durations.lock().unwrap().iter().sum::<Duration>()
                / (number * benchmarks.len()) as u32;

            println!("  mean duration:      {:?}", mean);
            let std_dev = (durations
                .lock()
                .unwrap()
                .iter()
                .map(|d| (d.as_nanos() as f64 - mean.as_nanos() as f64).powi(2))
                .sum::<f64>()
                / (number * benchmarks.len()) as f64)
                .sqrt();
            let std_dev = Duration::from_nanos(std_dev as u64);
            println!("  standard deviation: {:?}", std_dev);
            println!(
                "  fastest duration:   {:?}",
                durations.lock().unwrap().iter().min().unwrap()
            );
            println!(
                "  slowest duration:   {:?}",
                durations.lock().unwrap().iter().max().unwrap()
            );

            println!("latency distribution:");
            let mut durations = durations.lock().unwrap().clone();
            durations.sort();
            let pp = vec![99, 95, 90, 75, 50, 25, 10];
            for p in pp {
                println!("  {}%: {:?}", p, durations[durations.len() * p / 100]);
            }

            println!("latency histogram:");
            let (buckets, values) = histogram(&durations, 10);
            println!("  bin ranges:");
            for (start, end) in buckets {
                println!("  - [{:?}, {:?}]", start, end);
            }
            println!("  values:");
            let max_count = values.iter().max().unwrap_or(&0);
            let bar_scale = 50;
            let bars = values
                .iter()
                .map(|count| (count.to_string(), count * bar_scale / max_count))
                .collect::<Vec<_>>();
            let max_bar_len = bars.iter().map(|b| b.0.len()).max().unwrap_or(0);
            for (count, bar_len) in bars {
                let bar: String = "█".repeat(bar_len);
                println!("    {: >width$}: {}", count, bar, width = max_bar_len);
            }
        }
    }

    Ok(())
}

fn histogram(values: &Vec<Duration>, num_bins: usize) -> (Vec<(Duration, Duration)>, Vec<usize>) {
    let min = values.iter().min().unwrap().as_nanos();
    let max = values.iter().max().unwrap().as_nanos();
    let bin_size = (max - min) / num_bins as u128;

    let mut bins = vec![0; num_bins];
    for value in values {
        let mut bin = ((value.as_nanos() - min) / bin_size) as usize;
        if bin >= num_bins {
            bin = num_bins - 1;
        }
        bins[bin] += 1;
    }

    let mut bin_ranges = vec![];
    for i in 0..num_bins {
        let start = min + i as u128 * bin_size;
        let end = start + bin_size;
        bin_ranges.push((
            Duration::from_nanos(start as u64),
            Duration::from_nanos(end as u64),
        ));
    }
    (bin_ranges, bins)
}

pub async fn run_request(cfg: &Config, app: &mut Applicator, request: &str) -> Result<Response> {
    // Get the request by name and apply the context.
    let mut request: Request = match cfg.requests.get(request) {
        Some(r) => r.clone(),
        None => {
            return Err(anyhow::anyhow!("Request not found: {}", request));
        }
    };
    request.apply(app);

    // Make the requests.
    Ok(request.request().await?)
}

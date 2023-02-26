use std::{collections::HashMap, error::Error, fs::File, ops::RangeInclusive};

use benchmark::{BenchmarkResult, BenchmarkSettings, BenchmarkStats};
use clap::Parser;
use csv::Writer;
use indicatif::{ProgressBar, ProgressStyle};
use serde::Serialize;
use statrs::statistics::{OrderStatistics, Statistics};
use tabled::{Table, Tabled};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = 512, value_parser = connection_in_range)]
    connections: u16,

    #[arg(short, long, default_value_t = 100_000)]
    requests: u64,

    #[arg(short, long)]
    output_file: Option<String>,

    #[arg(short, long)]
    target_uri: String,
}

// THIS FUNCTIONS IS REFERENCED FROM AUTHOR
// If client and server run on the same machine and both use the loopback interface,
// We must allow at most 2**16 -1 (one for the server) connections, since each connection requires a port.
// We stay away from the maximum by a margin of 10
// We do not allow to run with zero commands

const CONNECTION_RANGE: RangeInclusive<usize> = 1..=65536 - 10;
fn connection_in_range(s: &str) -> Result<u16, String> {
    s.parse()
        .iter()
        .filter(|i| CONNECTION_RANGE.contains(i))
        .map(|i| *i as u16)
        .next()
        .ok_or(format!(
            "Number of connection not in range {}-{}",
            CONNECTION_RANGE.start(),
            CONNECTION_RANGE.end()
        ))
}

struct Progress {
    bar: ProgressBar,
}

impl BenchmarkStats for Progress {
    fn update(&self, n: u64) {
        self.bar.inc(n);
    }

    fn finish(&self) {
        self.bar.finish_and_clear();
    }
}

impl Progress {
    fn new(len: u64) -> Self {
        let bar = ProgressBar::new(len);
        bar.set_style(
            ProgressStyle::with_template("[{elapsed_precise}] {bar} {pos:>7}/{len:7} {msg}")
                .unwrap()
                .progress_chars("##-"),
        );
        Self { bar }
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let progress = Progress::new(args.requests.into());
    println!("Start benchmarking {}", &args.target_uri);
    let result = benchmark::run(
        progress,
        BenchmarkSettings {
            connections: args.connections,
            requests: args.requests,
            target_uri: benchmark::build_uri(&args.target_uri),
        },
    )
    .await;

    match result {
        Err(msg) => println!("error: {:?}", msg),
        Ok(summary) => {
            let output = process_result(summary);
            if let Some(file_path) = args.output_file {
                let _ = write_csv(file_path, output);
            } else {
                println!("{}", Table::new(output).to_string())
            }
        }
    }
}

#[derive(Debug, Tabled, Serialize)]
struct StatusStatistics {
    status: u16,
    requests: usize,
    #[tabled(display_with = "format_float")]
    min: f64,
    #[tabled(display_with = "format_float")]
    max: f64,
    #[tabled(display_with = "format_float")]
    mean: f64,
    #[tabled(display_with = "format_float")]
    std: f64,
    #[tabled(display_with = "format_float")]
    p90: f64,
    #[tabled(display_with = "format_float")]
    p99: f64,
}

fn format_float(num: &f64) -> String {
    format!("{:.2}", num)
}

fn process_result(summary: BenchmarkResult) -> Vec<StatusStatistics> {
    let mut status_latencies: HashMap<u16, Vec<f64>> = HashMap::new();
    for req_sum in summary.request_summaries {
        if let Some(status_statistic) = status_latencies.get_mut(&req_sum.status_code) {
            status_statistic.push(req_sum.latency.as_micros() as f64);
        } else {
            status_latencies.insert(
                req_sum.status_code,
                vec![req_sum.latency.as_micros() as f64],
            );
        }
    }

    let mut statistics: Vec<StatusStatistics> = vec![];
    for (key, val) in status_latencies.iter() {
        statistics.push(calculate_statistic(key, val));
    }

    statistics
}

fn calculate_statistic(status: &u16, latencies: &Vec<f64>) -> StatusStatistics {
    let min = latencies.min() / 1000f64;
    let max = latencies.max() / 1000f64;
    let mean = latencies.mean() / 1000f64;
    let variance = latencies.variance();
    let std = variance.sqrt() / 1000f64;
    let mut data = statrs::statistics::Data::new(latencies.clone());
    let p90 = data.percentile(90) / 1000f64;
    let p99 = data.percentile(99) / 1000f64;
    StatusStatistics {
        status: *status,
        requests: latencies.len(),
        min,
        max,
        mean,
        std,
        p90,
        p99,
    }
}

fn write_csv(path: String, records: Vec<StatusStatistics>) -> Result<(), Box<dyn Error>> {
    // Open a file to write the CSV output
    let file = File::create(path)?;

    // Create a CSV writer
    let mut writer = Writer::from_writer(file);

    // Write the header row
    writer.write_record(&[
        "status", "requests", "min", "max", "mean", "std", "p90", "p99",
    ])?;
    for x in records.iter() {
        writer.write_record(&[
            &x.status.to_string(),
            &x.requests.to_string(),
            &x.min.to_string(),
            &x.max.to_string(),
            &x.mean.to_string(),
            &x.std.to_string(),
            &x.p90.to_string(),
            &x.p99.to_string(),
        ])?;
    }

    // Flush the CSV writer to ensure all data is written
    writer.flush()?;

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_works_with_default_argument() {
        let args = Args::try_parse_from([
            "cli_load_test",
            "-t",
            "http://localhost:8080/person",
            "-o",
            "test.text",
        ])
        .unwrap();
        assert_eq!(args.connections, 512);
        assert_eq!(args.requests, 100_000);
        assert_eq!(args.target_uri, "http://localhost:8080/person");
        assert_eq!(args.output_file, Some(String::from("test.text")));
    }

    #[test]
    fn test_out_file_must_be_provided() {
        let result = Args::try_parse_from(["cli_load_test", "-t", "http://localhost:8080/person"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_connection_must_be_in_range() {
        let mut a = [
            "cli_load_test",
            "-t",
            "http://localhost:8080/person",
            "-o",
            "test.text",
            "-c",
            "placeholder",
        ];

        assert!(Args::try_parse_from({
            a[6] = "0";
            a
        })
        .is_err());

        assert!(Args::try_parse_from({
            a[6] = "-1";
            a
        })
        .is_err());

        assert!(Args::try_parse_from({
            a[6] = "65527"; // > 65536 - 10
            a
        })
        .is_err());
    }
}

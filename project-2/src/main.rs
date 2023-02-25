use std::{collections::HashMap, iter::Map, ops::RangeInclusive};

use benchmark::{BenchmarkResult, BenchmarkSettings, BenchmarkStats};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = 512, value_parser = connection_in_range)]
    connections: u16,

    #[arg(short, long, default_value_t = 100_000)]
    requests: u64,

    #[arg(short, long)]
    output_file: String,

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
        Ok(summary) => println!("{:?}", process_result(summary)),
        Err(msg) => println!("error: {:?}", msg),
    }
}

#[derive(Debug)]
struct StatusStatistics {
    min_latency: u128,
    max_latency: u128,
    mean_latency: u128,
    standard_deviation: u128,
    p90: u128,
    p99: u128,
    latencies: Vec<u128>,
}

impl StatusStatistics {
    fn first_record(t: u128) -> Self {
        Self {
            min_latency: 0,
            max_latency: 0,
            mean_latency: 0,
            standard_deviation: 0,
            p90: 0,
            p99: 0,
            latencies: vec![t],
        }
    }
}

fn process_result(summary: BenchmarkResult) -> HashMap<u16, StatusStatistics> {
    let mut statistics: HashMap<u16, StatusStatistics> = HashMap::new();
    for s in summary.request_summaries {
        if let Some(status_statistic) = statistics.get_mut(&s.status_code) {
            status_statistic.latencies.push(s.elapsed_micros);
        } else {
            statistics.insert(
                s.status_code,
                StatusStatistics::first_record(s.elapsed_micros),
            );
        }
    }

    for (_, s) in statistics.iter_mut() {
        s.min_latency = *s.latencies.iter().min().unwrap();
        s.max_latency = *s.latencies.iter().max().unwrap();
        s.mean_latency = s.latencies.iter().sum::<u128>() / s.latencies.len() as u128;
        s.standard_deviation = calculate_standard_deviation(&s.latencies);

        let mut sorted_measurements = s.latencies.clone();

        sorted_measurements.sort();
        s.p90 = sorted_measurements[(sorted_measurements.len() as f64 * 0.9).floor() as usize];
        s.p99 = sorted_measurements[(sorted_measurements.len() as f64 * 0.99).floor() as usize];
    }

    statistics
}

fn calculate_standard_deviation(values: &Vec<u128>) -> u128 {
    // Calculate the mean
    let mean = values.iter().sum::<u128>() as f64 / values.len() as f64;

    // Calculate the variance
    let variance = values
        .iter()
        .map(|value| (*value as f64 - mean).powf(2.0))
        .sum::<f64>()
        / values.len() as f64;

    // Take the square root of the variance to get the standard deviation
    let standard_deviation = variance.sqrt() as u128;

    standard_deviation
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
        assert_eq!(args.output_file, "test.text");
    }

    #[test]
    fn test_target_uri_must_be_provided() {
        let result = Args::try_parse_from(["cli_load_test", "-o", "test.text"]);
        assert!(result.is_err());
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

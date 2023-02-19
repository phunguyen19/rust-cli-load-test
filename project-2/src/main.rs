use std::ops::RangeInclusive;

use benchmark::BenchmarkSettings;
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

#[tokio::main]
async fn main() {
    let args = Args::parse();

    // let bar = ProgressBar::new(100);
    // bar.set_style(
    //     ProgressStyle::with_template("[{elapsed_precise}] {bar} {pos:>7}/{len:7} {msg}%")
    //         .unwrap()
    //         .progress_chars("##-"),
    // );
    // for _ in 0..100 {
    //     bar.inc(1);
    //     tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    // }
    // bar.finish();

    let result = benchmark::run(BenchmarkSettings {
        connections: args.connections,
        requests: args.requests,
        target_uri: benchmark::build_uri(&args.target_uri),
    })
    .await;

    match result {
        Ok(summary) => println!("summary: {:?}", summary),
        Err(msg) => println!("error: {:?}", msg),
    }
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

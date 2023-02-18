use benchmark::BenchmarkSettings;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    // number of connections
    #[arg(short, long, default_value_t = 512)]
    connections: u64,

    // number of requests sent
    #[arg(short, long, default_value_t = 100_000)]
    requests: u64,

    // output file (for Milestone 3), so the user has the option to save the results somewhere
    #[arg(short, long, default_value_t = String::from(""))]
    output_file: String,

    #[arg(short, long)]
    target_uri: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

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

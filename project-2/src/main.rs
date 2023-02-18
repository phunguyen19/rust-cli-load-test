use benchmark::BenchmarkSettings;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = 512)]
    connections: u64,

    #[arg(short, long, default_value_t = 100_000)]
    requests: u64,

    #[arg(short, long)]
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

#[cfg(test)]
mod test {
    // test connections
    // be number
    // greater than 0
    // smaller than max u65
    // is optional with default value

    // test requests
    // be number
    // greater than 0
    // smaller than max u65
    // greater than connections
    // is optional with default value

    // test output file
    // must be provided

    // test target uri
    // must be provided
    // must be valid url
}

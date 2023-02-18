use benchmark::BenchmarkSettings;

const CONNECTIONS: u64 = 10;
const REQUESTS_PER_CONNECTION: u64 = 10;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    let result = benchmark::run(BenchmarkSettings {
        connections: CONNECTIONS,
        requests_per_conn: REQUESTS_PER_CONNECTION,
        target_uri: benchmark::build_uri(&args[1]),
    })
    .await;

    match result {
        Ok(summary) => println!("summary: {:?}", summary),
        Err(msg) => println!("error: {:?}", msg),
    }
}

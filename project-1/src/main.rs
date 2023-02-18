use benchmark::BenchmarkSettings;

const CONNECTIONS: u64 = 512;
const REQUESTS: u64 = 100_000;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    let result = benchmark::run(BenchmarkSettings {
        connections: CONNECTIONS,
        requests: REQUESTS,
        target_uri: benchmark::build_uri(&args[1]),
    })
    .await;

    match result {
        Ok(summary) => println!("summary: {:?}", summary),
        Err(msg) => println!("error: {:?}", msg),
    }
}

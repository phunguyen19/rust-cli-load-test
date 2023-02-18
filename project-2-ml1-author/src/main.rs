use lp_loadcli_p2_mock::{BenchmarkSettings, run, BenchmarkResults};
use clap::Parser;

mod args;
use crate::args::Args;

#[tokio::main]
async fn main(){

    let args = Args::parse();    


    let settings = BenchmarkSettings{
        connections: args.connections as u64,
        requests: args.number_of_requests,
        target_uri: args.target_url,
    };

    println!("Running angainst {}",settings.target_uri);


    let BenchmarkResults{
        summaries,
        total_duration_ms: duration_ms
    } = run(settings.clone()).await.expect("The benchmark failed:");

    for result in &summaries{
        println!("con[{:4}]: ok: {}, ms: {}", result.connection_id, result.successful_requests, result.duration.as_millis())
    }

    let ok_requests :u64 = summaries.iter().map(|r| r.successful_requests).sum();
    let failed_requests :u64 = summaries.iter().map(|r| r.failed_requests).sum();
    let max_duration = summaries.iter().map(|r| r.duration.as_millis()).max().unwrap() as u64;
    
    println!("Performed {ok_requests} ({failed_requests} failed) requests with a maximum of {max_duration}ms");

    println!("Sent {} requests in {}ms", settings.requests, duration_ms);
    println!("This makes for {} req/s", settings.requests*1_000/duration_ms);

}


#[cfg(test)]
mod test{
    use clap::Parser;

    use crate::args::Args;

    #[test]
    fn test_works_without_arguments(){
        Args::try_parse_from(["loadcli"]).expect("Should work without arguments");
    }

    #[test]
    fn test_works_with_uri(){
        Args::try_parse_from([
            "loadcli",
            "https://some.host.example.com:4242/uri?param=foo&bar=bazz",
            ]).expect("Should work without arguments");
    }

    #[test]
    fn test_works_with_custom_connections(){
        let args = Args::try_parse_from([
            "loadcli",
            "-c", "1",
            "https://some.host.example.com:4242/uri?param=foo&bar=bazz",
            ]).expect("Should work without arguments");
        assert_eq!(args.connections,1)
    }
    #[test]
    fn test_argument_before_options(){
        let args = Args::try_parse_from([
            "loadcli",
            "https://some.host.example.com:4242/uri?param=foo&bar=bazz",
            "-c", "1",
            ]).expect("Should work without arguments");
        assert_eq!(args.connections,1)
    }
    #[test]
    fn test_rejects_invalid_connection_count(){
        let args = Args::try_parse_from([
            "loadcli",
            "-c", "0",
            ]);
        assert_eq!(args.is_err(),true);

        let args = Args::try_parse_from([
            "loadcli",
            "-c", "65533",
            ]);
        assert_eq!(args.is_err(),true);

        let args = Args::try_parse_from([
            "loadcli",
            "-c", "65536000",
            ]);
        assert_eq!(args.is_err(),true);
    }
    
}
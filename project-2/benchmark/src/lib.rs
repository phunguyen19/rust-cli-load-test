use std::{
    str::FromStr,
    time::{Duration, Instant},
};

use anyhow::{Context, Ok};
use async_trait::async_trait;
use hyper::{client::HttpConnector, Client, Uri};
use tokio::sync::mpsc::{channel, Receiver, Sender};

pub struct BenchmarkSettings {
    pub connections: u16,
    pub requests: u64,
    pub target_uri: Uri,
}

#[derive(Debug)]
pub struct BenchmarkResult {
    pub target_uri: Uri,
    pub total_time: Duration,
    pub request_summaries: Vec<RequestSummary>,
}

impl BenchmarkResult {
    pub fn new(target_uri: Uri) -> Self {
        Self {
            target_uri,
            total_time: Duration::from_secs(0),
            request_summaries: vec![],
        }
    }

    pub fn combine_conn_summaries(&mut self, conn_summaries: Vec<ConnectionSummary>) {
        for r in conn_summaries {
            self.request_summaries.extend(r.request_summaries);
        }
    }
}

#[derive(Debug)]
pub struct ConnectionSummary {
    total_requests: u64,
    success_requests: u64,
    fail_requests: u64,
    request_summaries: Vec<RequestSummary>,
}

#[derive(Debug)]
pub struct RequestSummary {
    pub latency: Duration,
    pub status_code: u16,
}

#[async_trait]
trait Requester {
    async fn get(&self, uri: Uri) -> anyhow::Result<u16>;
}

struct HttpClient(Client<HttpConnector>);

impl HttpClient {
    fn new() -> Self {
        HttpClient(Client::new())
    }
}

#[async_trait]
impl Requester for HttpClient {
    async fn get(&self, uri: Uri) -> anyhow::Result<u16> {
        let status = self.0.get(uri.clone()).await?.status().as_u16();
        Ok(status)
    }
}

struct ConnectionSettings {
    requests: u64,
    target_uri: Uri,
}

impl ConnectionSettings {
    fn from(value: &BenchmarkSettings) -> Self {
        Self {
            requests: value.requests / value.connections as u64,
            target_uri: value.target_uri.clone(),
        }
    }
}

pub fn build_uri(s: &String) -> Uri {
    Uri::from_str(s).expect("Unparsable target URI")
}

pub trait BenchmarkStats {
    fn update(&self, n: u64);
    fn finish(&self);
}

#[async_trait]
trait TaskStats {
    async fn update(&self, n: u64) -> ();
    async fn finish(&self) -> ();
}

struct TaskNotifier {
    tx: Sender<u64>,
}

#[async_trait]
impl TaskStats for TaskNotifier {
    async fn update(&self, n: u64) -> () {
        match self.tx.send(n).await {
            _ => (),
        }
    }

    async fn finish(&self) -> () {
        match self.tx.send(0).await {
            _ => (),
        }
    }
}

impl TaskNotifier {
    pub fn init_channel(buffer: usize) -> (Sender<u64>, Receiver<u64>) {
        channel(buffer)
    }
}

pub async fn run(
    process: impl BenchmarkStats,
    benchmark_settings: BenchmarkSettings,
) -> anyhow::Result<BenchmarkResult> {
    let mut result = BenchmarkResult::new(benchmark_settings.target_uri.clone());
    let (tx, mut rx) = TaskNotifier::init_channel(benchmark_settings.connections.into());

    let now = Instant::now();

    let mut conn_futures: Vec<_> = vec![];
    for _ in 0..benchmark_settings.connections {
        conn_futures.push(tokio::spawn(connection_task(
            HttpClient::new(),
            TaskNotifier { tx: tx.clone() },
            ConnectionSettings::from(&benchmark_settings),
        )));
    }

    let mut count_channel_closed = 0;
    loop {
        if let Some(n) = rx.recv().await {
            process.update(n);
            if n == 0 {
                count_channel_closed += 1;
            }
        }

        if count_channel_closed >= benchmark_settings.connections {
            break;
        }
    }

    result.total_time = now.elapsed();

    let mut conn_summaries: Vec<ConnectionSummary> = Vec::with_capacity(conn_futures.len());
    for f in conn_futures {
        let conn_future_result = f.await;
        let conn_summary_result = conn_future_result.context("Error spawning benchmark task")?;
        let conn_summary = conn_summary_result.context("Error making connection request")?;
        conn_summaries.push(conn_summary);
    }

    result.combine_conn_summaries(conn_summaries);

    process.finish();
    Ok(result)
}

async fn connection_task(
    client: impl Requester,
    stats: impl TaskStats,
    conn_setting: ConnectionSettings,
) -> anyhow::Result<ConnectionSummary> {
    let mut summary = ConnectionSummary {
        success_requests: 0,
        total_requests: 0,
        fail_requests: 0,
        request_summaries: vec![],
    };

    let mut queue_stats = 0;
    for _ in 0..conn_setting.requests {
        let now = Instant::now();
        let status_code = client.get(conn_setting.target_uri.clone()).await?;
        summary.request_summaries.push(RequestSummary {
            latency: now.elapsed(),
            status_code,
        });
        match status_code {
            200 => summary.success_requests += 1,
            _ => summary.fail_requests += 1,
        }

        summary.total_requests += 1;

        // send update stats
        // just send a batch instead
        // of send in every completed request
        queue_stats += 1;
        if queue_stats >= 199 {
            stats.update(queue_stats).await;
            queue_stats = 0;
        }
    }
    // send update stats
    // send remains in the queue
    if queue_stats > 0 {
        stats.update(queue_stats).await;
    }
    // notify finished
    stats.finish().await;

    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockHttpClient {
        status: Option<u16>,
    }

    impl MockHttpClient {
        fn with_status(status: Option<u16>) -> Self {
            Self { status }
        }
    }

    #[async_trait]
    impl Requester for MockHttpClient {
        async fn get(&self, _uri: Uri) -> anyhow::Result<u16> {
            match self.status {
                Some(status) => Ok(status),
                None => Err(anyhow::Error::msg("Test")),
            }
        }
    }

    struct MockTaskNotifier {}

    #[async_trait]
    impl TaskStats for MockTaskNotifier {
        async fn update(&self, _n: u64) -> () {
            ()
        }
        async fn finish(&self) -> () {
            ()
        }
    }

    fn mock_conn_settings() -> ConnectionSettings {
        ConnectionSettings {
            requests: 10,
            target_uri: Uri::from_static("abc"),
        }
    }

    #[tokio::test]
    async fn connection_task_success() {
        let result = connection_task(
            MockHttpClient::with_status(Some(200)),
            MockTaskNotifier {},
            mock_conn_settings(),
        )
        .await
        .expect("No error");

        assert_eq!(result.total_requests, 10);
        assert_eq!(result.success_requests, 10);
    }

    #[tokio::test]
    async fn connection_task_fail() {
        let result = connection_task(
            MockHttpClient::with_status(Some(500)),
            MockTaskNotifier {},
            mock_conn_settings(),
        )
        .await
        .expect("No error");

        assert_eq!(result.total_requests, 10);
        assert_eq!(result.success_requests, 0);
    }

    #[tokio::test]
    async fn connection_task_error() {
        let result = connection_task(
            MockHttpClient::with_status(None),
            MockTaskNotifier {},
            mock_conn_settings(),
        )
        .await;
        assert!(result.is_err());
    }
}

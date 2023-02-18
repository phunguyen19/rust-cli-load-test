use std::time::{Instant, Duration};
use hyper::Uri;
use anyhow::{Context, Result};
use async_trait::async_trait;
use rand_distr::{Distribution, Normal};
use rand::thread_rng;


#[derive(Clone)]
pub struct BenchmarkSettings{
    pub connections: u64,
    pub requests: u64,
    pub target_uri: Uri,
}
pub struct BenchmarkResults{
    pub summaries: Vec<ConnectionSummary>,
    pub total_duration_ms: u64,
}

pub struct ConnectionSummary{
    pub connection_id: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub duration: Duration,
}
impl ConnectionSummary{
    fn new(connection_id: u64)->Self{
        ConnectionSummary { 
            connection_id, 
            successful_requests: 0, 
            failed_requests: 0, 
            duration: Duration::default(),
        }
    }
}

struct ConnectionSettings{
    connection_id: u64,
    target_uri: Uri,
    num_requests: u64,
}


pub async fn run(settings: BenchmarkSettings) -> Result<BenchmarkResults>{

    let mut handles = Vec::with_capacity(settings.connections as usize);
    let mut clients = Vec::with_capacity(settings.connections as usize);
    let mut all_results = Vec::with_capacity(settings.connections as usize);


    for _ in 0..settings.connections{
        let client = WaitingHttpClient;
        clients.push(client);
    }

    let start_instant = Instant::now();

    for (id,c) in clients.into_iter().enumerate() {
        let settings = ConnectionSettings{
                connection_id: id as u64, 
                target_uri: settings.target_uri.clone(), 
                num_requests: settings.requests/settings.connections
            };
        let h = tokio::spawn(connection_task(c, settings ));
        handles.push(h)
    }

    for h in handles{
        let await_result = h.await;
        let connection_result = await_result.context("Failed to await for task")?;
        let result = connection_result.context("A connection failed")?;
        all_results.push(result);
    }

    let duration_ms = start_instant.elapsed().as_millis() as u64;

    Ok(BenchmarkResults{
        summaries: all_results,
        total_duration_ms: duration_ms
    })
}

pub struct WaitingHttpClient;

#[async_trait]
pub trait StatusOnlyHttpClient{
    async fn get(&self, uri: Uri) -> Result<u16>;
}

#[async_trait]
impl StatusOnlyHttpClient for WaitingHttpClient{
    async fn get(&self, _uri: Uri) -> Result<u16> {

        let v = {
            let mut rng = thread_rng();
            let normal = Normal::new(20.0, 6.0)?;
            normal.sample(&mut rng)
        }; 
        // This does not actually work, see the solution for more details
        tokio::time::sleep(Duration::from_micros(v as u64)).await;
        Ok(200)
    }
}
async fn connection_task(client: impl StatusOnlyHttpClient, settings: ConnectionSettings ) -> Result<ConnectionSummary>{

    let mut summary = ConnectionSummary::new(settings.connection_id);
    let start_instant = Instant::now();
    for _ in 0..settings.num_requests{
        let status = client
            .get(settings.target_uri.clone())
            .await
            .context("A request failed")?; // don't worry, we take care of this shortly
        if status < 408 {
            summary.successful_requests+=1;
        }
        else{
            summary.failed_requests+=1;
        }
    }  
    let duration = start_instant.elapsed();
    summary.duration = duration;
    Ok(summary)
}


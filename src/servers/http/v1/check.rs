use std::net::SocketAddr;
use std::time::Duration;

use derive_more::derive::Display;
use futures::Stream;
use tokio::task::JoinSet;
use tokio::time::error::Elapsed;
use torrust_tracker_services::registration::{ServiceHealthCheck, ServiceHeathCheckResult};

/// Checks the Health by connecting to the HTTP tracker endpoint.
///
#[derive(Debug, Display)]
#[display("Check: {local_addr}, {info}")]
pub struct HealthCheck {
    local_addr: SocketAddr,
    info: String,
    requests: JoinSet<Result<Result<reqwest::Response, reqwest::Error>, Elapsed>>,
}

impl HealthCheck {
    pub fn new(local_addr: SocketAddr, info: String) -> Self {
        Self {
            local_addr,
            info,
            requests: JoinSet::new(),
        }
    }

    fn new_request(&mut self) {
        let url = format!("http://{}/health_check", self.local_addr()); // DevSkim: ignore DS137138

        self.requests
            .spawn(tokio::time::timeout(Duration::from_secs(5), reqwest::get(url)));
    }
}

impl ServiceHealthCheck for HealthCheck {
    fn local_addr(&self) -> std::net::SocketAddr {
        self.local_addr
    }

    fn info(&self) -> String {
        self.info.clone()
    }
}

impl Stream for HealthCheck {
    type Item = ServiceHeathCheckResult;

    fn poll_next(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        let ready = match self.requests.poll_join_next(cx) {
            std::task::Poll::Ready(ready) => ready,
            std::task::Poll::Pending => return std::task::Poll::Pending,
        };

        let reply = ServiceHeathCheckResult::new(match ready {
            Some(Ok(Ok(Ok(response)))) => Ok(response.status().to_string()),
            Some(Ok(Ok(Err(e)))) => Err(e.to_string()),
            Some(Ok(Err(e))) => Err(e.to_string()),
            Some(Err(e)) => Err(e.to_string()),
            None => {
                self.new_request();
                cx.waker().wake_by_ref();
                return std::task::Poll::Pending;
            }
        });

        std::task::Poll::Ready(Some(reply))
    }
}

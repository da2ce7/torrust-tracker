use std::net::SocketAddr;
use std::ops::Deref;
use std::time::Duration;

use derive_more::derive::{Constructor, Display};
use futures::Stream;
use thiserror::Error;
use tokio::task::JoinSet;
use tokio::time::error::Elapsed;
use torrust_tracker_services::{ServiceCheck, ServiceCheckResult};

#[derive(Debug, Error)]

pub enum Error {
    #[error("Failed to get successful response {0}")]
    Request(reqwest::Error),
    #[error("Timed out while waiting for response {0}")]
    Timeout(Elapsed),
    #[error("Failed to join task successfully (maybe panic) {0}")]
    Join(tokio::task::JoinError),
}

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
        let url = format!("http://{}/health_check", self.local_addr); // DevSkim: ignore DS137138

        self.requests
            .spawn(tokio::time::timeout(Duration::from_secs(5), reqwest::get(url)));
    }
}
#[derive(Debug, Constructor)]
struct CheckResult(Result<String, Error>);

impl std::fmt::Display for CheckResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            Ok(success) => write!(f, "Success: {success}"),
            Err(error) => write!(f, "Failure: {error}"),
        }
    }
}

impl ServiceCheckResult for CheckResult {
    type Success = String;
    type Error = Error;
}

impl Deref for CheckResult {
    type Target = Result<<Self as ServiceCheckResult>::Success, <Self as ServiceCheckResult>::Error>;

    fn deref(&self) -> &Self::Target {
        todo!()
    }
}

impl ServiceCheck<CheckResult> for HealthCheck {}

impl Stream for HealthCheck {
    type Item = CheckResult;

    fn poll_next(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        let ready = match self.requests.poll_join_next(cx) {
            std::task::Poll::Ready(ready) => ready,
            std::task::Poll::Pending => return std::task::Poll::Pending,
        };

        let reply = Self::Item::new(match ready {
            Some(Ok(Ok(Ok(response)))) => Ok(response.status().to_string()),
            Some(Ok(Ok(Err(e)))) => Err(Error::Request(e)),
            Some(Ok(Err(e))) => Err(Error::Timeout(e)),
            Some(Err(e)) => Err(Error::Join(e)),
            None => {
                self.new_request();
                cx.waker().wake_by_ref();
                return std::task::Poll::Pending;
            }
        });

        std::task::Poll::Ready(Some(reply))
    }
}

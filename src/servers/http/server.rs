use std::net::SocketAddr;
use std::time::{Duration, Instant};

use derive_more::derive::Constructor;
use futures::future::BoxFuture;
use futures::{FutureExt, Stream, StreamExt as _};
use tokio::task::JoinSet;
use tokio::time::sleep;
use torrust_tracker_services::registration::Registration;
use torrust_tracker_services::service::Service;

use super::error::Error;
use super::launcher::{Launcher, ProtocolTls};
use crate::servers::http::v1::HealthCheck;
use crate::servers::http::HTTP_TRACKER_LOG_TARGET;
use crate::servers::logging::STARTED_ON;

pub struct Server {
    launcher: Launcher,
    running: Option<Running<Server>>,
}

impl Service for Server {
    type Launcher = Launcher;
    type Error = Error;

    fn new(launcher: Self::Launcher) -> Result<Self, Self::Error> {
        Ok(Server { launcher, running: None })
    }

    fn start<'a>(mut self) -> Result<BoxFuture<'a, Result<(Self, Registration), Self::Error>>, Self::Error> {
        let started = self.launcher.start()?;

        // important!
        let local_udp_url = format!("{}://{}", started.protocol, started.local_addr);
        tracing::info!(target: HTTP_TRACKER_LOG_TARGET, "{STARTED_ON}: {local_udp_url}");

        let registration = Registration::new(HealthCheck::new(
            started.local_addr,
            "health check for http tracker".to_string(),
        ));

        self.running = Some(started);
        Ok(std::future::ready(Ok((self, registration))).boxed())
    }

    fn stop<'a>(mut self) -> Result<BoxFuture<'a, Result<Self, Self::Error>>, Self::Error> {
        let Some(started) = self.running.take() else {
            return Err(Error::NotStarted);
        };

        Ok(async move {
            let () = started.stop().await?;

            Ok(self)
        }
        .boxed())
    }
}

impl Stream for Server {
    type Item = Error;

    fn poll_next(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        let Some(service) = &mut self.running else {
            return std::task::Poll::Ready(None);
        };

        service.poll_next_unpin(cx)
    }
}

#[derive(Debug, Constructor)]
pub(super) struct Running<S>
where
    S: Service,
{
    tasks: JoinSet<Result<(), S::Error>>,
    handle: axum_server::Handle,
    shutdown_timeout: Option<Duration>,
    local_addr: SocketAddr,
    protocol: ProtocolTls,
}

impl Running<Server> {
    async fn stop(mut self) -> Result<(), <Server as Service>::Error> {
        let () = Self::shutdown(self.handle, self.shutdown_timeout).await;

        let timeout_at = Instant::now().checked_add(Duration::from_millis(500)).unwrap();

        while let Some(task) = {
            match tokio::time::timeout_at(timeout_at.into(), self.tasks.join_next()).await {
                Ok(task) => task,
                Err(_timeout) => {
                    tracing::warn!("timeout deadline reached");
                    self.tasks.shutdown().await;
                    None
                }
            }
        } {
            match task {
                Ok(Ok(())) => (),
                Ok(Err(e)) => return Err(e),

                Err(e) => return Err(Error::TaskPanicked(e)),
            }
        }

        Ok(())
    }

    async fn shutdown(handle: axum_server::Handle, shutdown_timeout: Option<Duration>) {
        if let Some(timeout) = shutdown_timeout {
            tracing::debug!("Sending graceful shutdown signal");
            handle.graceful_shutdown(Some(timeout));

            let now = Instant::now();

            if handle.connection_count() == 0 {
                tracing::debug!("no active connections... shutting down");
            } else {
                loop {
                    if handle.connection_count() == 0 {
                        tracing::debug!("no more connections... shutting down");
                        break;
                    }

                    if now.elapsed() > timeout {
                        tracing::warn!(remaining_alive_connections= %handle.connection_count(), "timed out... shutting down");
                        break;
                    }
                    tracing::info!(remaining_alive_connections= %handle.connection_count(), elapsed= ?now.elapsed(), "waiting...");

                    sleep(Duration::from_secs(1)).await;
                }
            }
        }

        handle.shutdown();
    }
}

impl Stream for Running<Server> {
    type Item = <Server as Service>::Error;

    fn poll_next(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        let task = match self.tasks.poll_join_next(cx) {
            std::task::Poll::Ready(Some(task)) => task,
            std::task::Poll::Ready(None) => return std::task::Poll::Ready(None),
            std::task::Poll::Pending => return std::task::Poll::Pending,
        };

        std::task::Poll::Ready(match task {
            Ok(Ok(())) => Some(Error::TaskUnexpectedlyQuit),
            Ok(Err(e)) => Some(e),
            Err(e) => Some(Error::TaskUnexpectedlyPanicked(e)),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use torrust_tracker_test_helpers::configuration::ephemeral_public;

    use crate::bootstrap::app::initialize_with_configuration;
    use crate::bootstrap::jobs::make_rust_tls;
    use crate::servers::http::server::{Launcher, Server};
    use crate::servers::registar::Registar;

    #[tokio::test]
    async fn it_should_be_able_to_start_and_stop() {
        let cfg = Arc::new(ephemeral_public());
        let tracker = initialize_with_configuration(&cfg);
        let http_trackers = cfg.http_trackers.clone().expect("missing HTTP trackers configuration");
        let config = &http_trackers[0];

        let bind_to = config.bind_address;

        let tls = make_rust_tls(&config.tsl_config)
            .await
            .map(|tls| tls.expect("tls config failed"));

        let register = &Registar::default();

        let stopped = Server::new(Launcher::new(tracker, tls, bind_to));
        let mut started = stopped.start(register.give_form()).expect("it should start the server");
        let () = started.stop().expect("it should stop the server");

        let stopped = started.await.expect("it should not shutdown with an error");

        assert_eq!(stopped.state.launcher.bind_to, bind_to);
    }
}

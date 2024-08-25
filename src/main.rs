use torrust_tracker::{app, bootstrap};

#[tokio::main]
async fn main() {
    let (config, tracker) = bootstrap::app::setup();

    let mut jobs = app::start(&config, tracker).await;

    tokio::signal::ctrl_c().await.expect("the signal should not error");

    while let Some(task) = jobs.join_next().await {
        match task {
            Ok(()) => (),
            Err(e) => tracing::warn!(%e, "task did not shutdown cleanly"),
        }
    }
}

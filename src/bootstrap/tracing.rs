//! Setup for the application tracing.
//!
//! It redirects the tracing info to the standard output with the tracing level defined in the configuration.
//!
//! - `Off` (i.e. don't load any subscriber...)
//! - `Error`
//! - `Warn`
//! - `Info`
//! - `Debug`
//! - `Trace`
//!
//! Refer to the [configuration crate documentation](https://docs.rs/torrust-tracker-configuration) to know how to change tracing settings.
use std::sync::Once;

use torrust_tracker_configuration::{Configuration, TraceLevel};
use tracing::info;

static INIT: Once = Once::new();

/// It redirects the tracing info to the standard output with the tracing level defined in the configuration
pub fn setup(cfg: &Configuration) {
    let level = config_level_or_default(&cfg.tracing_max_verbosity_level);

    if level.is_none() {
        return;
    }

    INIT.call_once(|| {
        stdout_config(level);
    });
}

fn config_level_or_default(trace_level: &TraceLevel) -> Option<tracing::Level> {
    let level = trace_level.to_string();

    if level == "off" {
        return None;
    };

    if let Ok(level) = level.parse() {
        return Some(level);
    }

    // Otherwise We Use Default
    config_level_or_default(&TraceLevel::default())
}

fn stdout_config(level: Option<tracing::Level>) {
    let () = tracing_subscriber::fmt().pretty().with_max_level(level).init();

    info!("tracing initialized.");
}

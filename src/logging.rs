use std::sync::Once;

use log::{info, LevelFilter};

use crate::settings::GlobalSettings;

static INIT: Once = Once::new();

pub fn setup_logging(settings: &GlobalSettings) {
    let level = settings.get_log_filter_level();

    if level == log::LevelFilter::Off {
        return;
    }

    INIT.call_once(|| {
        stdout_settings(level);
    });
}

fn stdout_settings(level: LevelFilter) {
    if let Err(_err) = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{} [{}][{}] {}",
                chrono::Local::now().format("%+"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(level)
        .chain(std::io::stdout())
        .apply()
    {
        panic!("Failed to initialize logging.")
    }

    info!("logging initialized.");
}

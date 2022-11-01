pub mod api;
pub mod databases;
pub mod errors;
pub mod http;
pub mod jobs;
pub mod logging;
pub mod old_settings;
pub mod protocol;
pub mod settings;
pub mod setup;
pub mod tracker;
pub mod udp;

pub mod config_const {
    pub const CONFIG_FOLDER: &str = "config";
    pub const CONFIG_DEFAULT: &str = "default";
    pub const CONFIG_LOCAL: &str = "local";
    pub const CONFIG_OVERRIDE: &str = "override";
    pub const CONFIG_OLD_LOCAL: &str = "../config";
}

#[macro_use]
extern crate lazy_static;

pub mod static_time {
    use std::time::SystemTime;

    lazy_static! {
        pub static ref TIME_AT_APP_START: SystemTime = SystemTime::now();
    }
}

pub mod ephemeral_instance_keys {
    use rand::rngs::ThreadRng;
    use rand::Rng;

    pub type Seed = [u8; 32];

    lazy_static! {
        pub static ref RANDOM_SEED: Seed = Rng::gen(&mut ThreadRng::default());
    }
}

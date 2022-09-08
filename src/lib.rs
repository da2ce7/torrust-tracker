pub use http::server::*;
pub use udp::server::*;

pub use self::config::*;
pub use self::tracker::*;
pub use api::server::*;
pub use protocol::common::*;

pub mod api;
pub mod config;
pub mod databases;
pub mod http;
pub mod jobs;
pub mod logging;
pub mod protocol;
pub mod setup;
pub mod tracker;
pub mod udp;

#[macro_use]
extern crate lazy_static;

pub mod keys {

    mod private {

        type Blowfish = blowfish::Blowfish<byteorder::LE>;

        lazy_static! {
            pub static ref BLOWFISH: Blowfish = <Blowfish as crypto_common::KeyInit>::new(
                &<Blowfish as crypto_common::KeyInit>::generate_key(rand::rngs::ThreadRng::default())
            );
        }

        lazy_static! {
            pub static ref TEST_BLOWFISH: Blowfish = <Blowfish as crypto_common::KeyInit>::new(
                &<Blowfish as crypto_common::KeyInit>::generate_key(<rand::rngs::StdRng as rand::SeedableRng>::seed_from_u64(0))
            );
        }

        lazy_static! {
            pub static ref SECRET: [u8; 32] = rand::Rng::gen(&mut rand::rngs::ThreadRng::default());
        }

        lazy_static! {
            pub static ref TEST_SECRET: [u8; 32] =
                rand::Rng::gen(&mut <rand::rngs::StdRng as rand::SeedableRng>::seed_from_u64(0));
        }
    }

    #[cfg(not(test))]
    pub use private::BLOWFISH as DEFAULT_KEY;

    #[cfg(test)]
    pub use private::TEST_BLOWFISH as DEFAULT_KEY;

    #[cfg(not(test))]
    pub use private::SECRET as DEFAULT_SECRET;

    #[cfg(test)]
    pub use private::TEST_SECRET as DEFAULT_SECRET;

    pub fn initialize_default_key() {
        lazy_static::initialize(&DEFAULT_KEY);
    }

    pub fn initialize_default_secret() {
        lazy_static::initialize(&DEFAULT_SECRET);
    }
}

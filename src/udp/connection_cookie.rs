use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::net::SocketAddr;

use aquatic_udp_protocol::ConnectionId;
use cipher::{BlockDecrypt, BlockEncrypt};

use crate::protocol::clock::time_extent::{DefaultTimeExtentMaker, Extent, MakeTimeExtent, TimeExtent};
use crate::protocol::clock::{DefaultClock, Time, TimeNow};
use crate::protocol::crypto::keys::block_ciphers::{BlockCipherKeeper, CipherArray, DefaultBlockCipher};
use crate::protocol::crypto::keys::seeds::{DefaultSeed, SeedKeeper};
use crate::udp::ServerError;

pub type Cookie = [u8; 8];

pub type SinceUnixEpochTimeExtent = TimeExtent;

pub const COOKIE_LIFETIME: TimeExtent = TimeExtent::from_sec(2, &60);

pub fn from_connection_id(connection_id: &ConnectionId) -> Cookie {
    connection_id.0.to_le_bytes()
}

pub fn into_connection_id(connection_cookie: &Cookie) -> ConnectionId {
    ConnectionId(i64::from_le_bytes(*connection_cookie))
}

pub trait ConnectionCookie {
    fn make_connection_cookie(remote_address: &SocketAddr) -> Cookie;
    fn check_connection_cookie(
        remote_address: &SocketAddr,
        connection_cookie: &Cookie,
    ) -> Result<SinceUnixEpochTimeExtent, ServerError>;
}

pub type DefaultConnectionCookie = EncryptedConnectionCookie;

pub struct HashedConnectionCookie;

impl ConnectionCookie for HashedConnectionCookie {
    fn make_connection_cookie(remote_address: &SocketAddr) -> Cookie {
        let time_extent = HashedConnectionCookie::get_last_time_extent();

        HashedConnectionCookie::build(remote_address, &time_extent)
    }

    fn check_connection_cookie(
        remote_address: &SocketAddr,
        connection_cookie: &Cookie,
    ) -> Result<SinceUnixEpochTimeExtent, ServerError> {
        // we loop backwards testing each time_extent until we find one that matches.
        // (or the lifetime of time_extents is exhausted)
        for offset in 0..=COOKIE_LIFETIME.amount {
            let checking_time_extent = HashedConnectionCookie::get_last_time_extent().decrease(offset).unwrap();

            let checking_cookie = HashedConnectionCookie::build(remote_address, &checking_time_extent);

            if *connection_cookie == checking_cookie {
                return Ok(checking_time_extent);
            }
        }
        Err(ServerError::InvalidConnectionId)
    }
}

impl HashedConnectionCookie {
    fn get_last_time_extent() -> SinceUnixEpochTimeExtent {
        DefaultTimeExtentMaker::now(&COOKIE_LIFETIME.increment)
            .unwrap()
            .unwrap()
            .increase(COOKIE_LIFETIME.amount)
            .unwrap()
    }

    fn build(remote_address: &SocketAddr, time_extent: &TimeExtent) -> Cookie {
        let seed = DefaultSeed::get_seed();

        let mut hasher = DefaultHasher::new();

        remote_address.hash(&mut hasher);
        time_extent.hash(&mut hasher);
        seed.hash(&mut hasher);

        hasher.finish().to_le_bytes()
    }
}

pub struct WitnessConnectionCookie;

impl ConnectionCookie for WitnessConnectionCookie {
    fn make_connection_cookie(remote_address: &SocketAddr) -> Cookie {
        // get the time that the cookie will expire.
        let expiry_time = DefaultClock::add(&COOKIE_LIFETIME.total().unwrap().unwrap())
            .unwrap()
            .as_secs_f32()
            .to_le_bytes();

        WitnessConnectionCookie::build(remote_address, expiry_time)
    }

    fn check_connection_cookie(
        remote_address: &SocketAddr,
        connection_cookie: &Cookie,
    ) -> Result<SinceUnixEpochTimeExtent, ServerError> {
        let expiry_time = WitnessConnectionCookie::extract_time(connection_cookie);
        let time_clock = DefaultClock::now().as_secs_f32();

        //println!("expiry_time: {expiry_time:?}, time_clock: {time_clock:?}");

        // lets check if the cookie has expired.
        if expiry_time < time_clock {
            return Err(ServerError::ExpiredConnectionId);
        }

        if *connection_cookie != WitnessConnectionCookie::build(remote_address, expiry_time.to_le_bytes()) {
            return Err(ServerError::BadWitnessConnectionId);
        }

        Ok(TimeExtent::from_sec(1, &(expiry_time.round() as u64)))
    }
}

impl WitnessConnectionCookie {
    fn build(remote_address: &SocketAddr, expiry: [u8; 4]) -> Cookie {
        let seed = DefaultSeed::get_seed();

        let mut hasher = DefaultHasher::new();

        remote_address.hash(&mut hasher);
        seed.hash(&mut hasher);
        expiry.hash(&mut hasher);

        let witness = hasher.finish().to_le_bytes();

        [
            witness[0], witness[1], witness[2], witness[3], expiry[0], expiry[1], expiry[2], expiry[3],
        ]
    }
    fn extract_time(cookie: &Cookie) -> f32 {
        f32::from_le_bytes([cookie[4], cookie[5], cookie[6], cookie[7]])
    }
}

pub struct EncryptedConnectionCookie;

impl ConnectionCookie for EncryptedConnectionCookie {
    fn make_connection_cookie(remote_address: &SocketAddr) -> Cookie {
        // get the time that the cookie will expire.
        let expiry_time = DefaultClock::add(&COOKIE_LIFETIME.total().unwrap().unwrap())
            .unwrap()
            .as_secs_f32()
            .to_le_bytes();

        EncryptedConnectionCookie::build(&EncryptedConnectionCookie::get_remote_hash(remote_address), expiry_time)
    }

    fn check_connection_cookie(
        remote_address: &SocketAddr,
        connection_cookie: &Cookie,
    ) -> Result<SinceUnixEpochTimeExtent, ServerError> {
        let cookie = EncryptedConnectionCookie::decode_cookie(connection_cookie);
        let expiry_time = EncryptedConnectionCookie::extract_time(&cookie);
        let time_clock = DefaultClock::now().as_secs_f32();
        let time_clock_expiry = DefaultClock::add(&COOKIE_LIFETIME.total().unwrap().unwrap())
            .unwrap()
            .as_secs_f32();
        let remote_hash = EncryptedConnectionCookie::get_remote_hash(remote_address);

        //println!("expiry_time: {expiry_time:?}, time_clock: {time_clock:?}");

        // cannot to be too old.
        if expiry_time < time_clock {
            return Err(ServerError::ExpiredConnectionId);
        }

        // or impossibly new.
        if expiry_time > time_clock_expiry {
            return Err(ServerError::BadWitnessConnectionId);
        }

        // and the remote hash should match
        if cookie[0..4] != remote_hash[0..4] {
            return Err(ServerError::BadWitnessConnectionId);
        }

        Ok(TimeExtent::from_sec(1, &(expiry_time.round() as u64)))
    }
}

impl EncryptedConnectionCookie {
    fn get_remote_hash(remote_address: &SocketAddr) -> [u8; 8] {
        let mut hasher = DefaultHasher::new();

        remote_address.hash(&mut hasher);

        hasher.finish().to_le_bytes()
    }

    fn build(remote_hash: &[u8; 8], expiry: [u8; 4]) -> Cookie {
        let mut array = CipherArray::from([
            remote_hash[0],
            remote_hash[1],
            remote_hash[2],
            remote_hash[3],
            expiry[0],
            expiry[1],
            expiry[2],
            expiry[3],
        ]);

        DefaultBlockCipher::get_block_cipher().encrypt_block(&mut array);

        array.into()
    }

    fn decode_cookie(encoded_cookie: &Cookie) -> Cookie {
        let mut array = CipherArray::from(*encoded_cookie);

        DefaultBlockCipher::get_block_cipher().decrypt_block(&mut array);

        array.into()
    }

    fn extract_time(cookie: &Cookie) -> f32 {
        f32::from_le_bytes([cookie[4], cookie[5], cookie[6], cookie[7]])
    }
}

#[cfg(test)]
mod tests {
    mod hashed_connection_cookie {
        use std::net::{IpAddr, Ipv4Addr, SocketAddr};

        use crate::protocol::clock::time_extent::Extent;
        use crate::protocol::clock::{StoppedClock, StoppedTime};
        use crate::udp::connection_cookie::{
            ConnectionCookie, Cookie, HashedConnectionCookie as TestConnectionCookie, COOKIE_LIFETIME,
        };

        #[test]
        fn it_should_make_a_connection_cookie() {
            let remote_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);
            let cookie = TestConnectionCookie::make_connection_cookie(&remote_address);

            // Note: This constant may need to be updated in the future as the hash is not guaranteed to to be stable between versions.
            const ID_COOKIE: Cookie = [23, 204, 198, 29, 48, 180, 62, 19];

            assert_eq!(cookie, ID_COOKIE)
        }

        #[test]
        fn it_should_make_different_cookies_for_the_next_time_extent() {
            let remote_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);
            let cookie = TestConnectionCookie::make_connection_cookie(&remote_address);

            StoppedClock::local_add(&COOKIE_LIFETIME.increment).unwrap();

            let cookie_next = TestConnectionCookie::make_connection_cookie(&remote_address);

            assert_ne!(cookie, cookie_next)
        }

        #[test]
        fn it_should_be_valid_for_this_time_extent() {
            let remote_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);

            let cookie = TestConnectionCookie::make_connection_cookie(&remote_address);

            TestConnectionCookie::check_connection_cookie(&remote_address, &cookie).unwrap();
        }

        #[test]
        fn it_should_be_valid_for_the_next_time_extent() {
            let remote_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);

            let cookie = TestConnectionCookie::make_connection_cookie(&remote_address);

            StoppedClock::local_add(&COOKIE_LIFETIME.increment).unwrap();

            TestConnectionCookie::check_connection_cookie(&remote_address, &cookie).unwrap();
        }

        #[test]
        fn it_should_be_valid_for_the_last_time_extent() {
            let remote_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);

            let cookie = TestConnectionCookie::make_connection_cookie(&remote_address);

            StoppedClock::local_set(&COOKIE_LIFETIME.total().unwrap().unwrap());

            TestConnectionCookie::check_connection_cookie(&remote_address, &cookie).unwrap();
        }

        #[test]
        #[should_panic]
        fn it_should_be_not_valid_after_their_last_time_extent() {
            let remote_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);

            let cookie = TestConnectionCookie::make_connection_cookie(&remote_address);

            StoppedClock::local_set(&COOKIE_LIFETIME.total_next().unwrap().unwrap());

            TestConnectionCookie::check_connection_cookie(&remote_address, &cookie).unwrap();
        }

        mod detail {
            use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

            use crate::protocol::clock::time_extent;
            use crate::udp::connection_cookie::HashedConnectionCookie;

            #[test]
            fn it_should_build_the_same_connection_cookie_for_the_same_input_data() {
                let remote_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);
                let time_extent_zero = time_extent::ZERO;

                let cookie = HashedConnectionCookie::build(&remote_address, &time_extent_zero);
                let cookie_2 = HashedConnectionCookie::build(&remote_address, &time_extent_zero);

                assert_eq!(cookie, cookie_2)
            }

            #[test]
            fn it_should_build_the_different_connection_cookie_for_different_ip() {
                let remote_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);
                let remote_address_2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::BROADCAST), 0);
                let time_extent_zero = time_extent::ZERO;

                let cookie = HashedConnectionCookie::build(&remote_address, &time_extent_zero);
                let cookie_2 = HashedConnectionCookie::build(&remote_address_2, &time_extent_zero);

                assert_ne!(cookie, cookie_2)
            }

            #[test]
            fn it_should_build_the_different_connection_cookie_for_different_ip_version() {
                let remote_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);
                let remote_address_2 = SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0);
                let time_extent_zero = time_extent::ZERO;

                let cookie = HashedConnectionCookie::build(&remote_address, &time_extent_zero);
                let cookie_2 = HashedConnectionCookie::build(&remote_address_2, &time_extent_zero);

                assert_ne!(cookie, cookie_2)
            }

            #[test]
            fn it_should_build_the_different_connection_cookie_for_different_socket() {
                let remote_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);
                let remote_address_2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 1);
                let time_extent_zero = time_extent::ZERO;

                let cookie = HashedConnectionCookie::build(&remote_address, &time_extent_zero);
                let cookie_2 = HashedConnectionCookie::build(&remote_address_2, &time_extent_zero);

                assert_ne!(cookie, cookie_2)
            }

            #[test]
            fn it_should_build_the_different_connection_cookie_for_different_time_extents() {
                let remote_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);
                let time_extent_zero = time_extent::ZERO;
                let time_extent_max = time_extent::MAX;

                let cookie = HashedConnectionCookie::build(&remote_address, &time_extent_zero);
                let cookie_2 = HashedConnectionCookie::build(&remote_address, &time_extent_max);

                assert_ne!(cookie, cookie_2)
            }
        }
    }
    mod witness_connection_cookie {
        use std::net::{IpAddr, Ipv4Addr, SocketAddr};

        use crate::protocol::clock::time_extent::Extent;
        use crate::protocol::clock::{StoppedClock, StoppedTime};
        use crate::udp::connection_cookie::{
            ConnectionCookie, Cookie, WitnessConnectionCookie as TestConnectionCookie, COOKIE_LIFETIME,
        };

        #[test]
        fn it_should_make_a_connection_cookie() {
            let remote_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);
            let cookie = TestConnectionCookie::make_connection_cookie(&remote_address);

            // Note: This constant may need to be updated in the future as the hash is not guaranteed to to be stable between versions.
            const ID_COOKIE: Cookie = [72, 245, 201, 136, 0, 0, 240, 66];

            assert_eq!(cookie, ID_COOKIE)
        }

        #[test]
        fn it_should_make_different_cookies_for_the_next_time_extent() {
            let remote_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);

            let cookie = TestConnectionCookie::make_connection_cookie(&remote_address);

            StoppedClock::local_add(&COOKIE_LIFETIME.increment).unwrap();

            let cookie_next = TestConnectionCookie::make_connection_cookie(&remote_address);

            assert_ne!(cookie, cookie_next)
        }

        #[test]
        fn it_should_be_valid_for_this_time_extent() {
            let remote_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);

            let cookie = TestConnectionCookie::make_connection_cookie(&remote_address);

            TestConnectionCookie::check_connection_cookie(&remote_address, &cookie).unwrap();
        }

        #[test]
        fn it_should_be_valid_for_the_next_time_extent() {
            let remote_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);

            let cookie = TestConnectionCookie::make_connection_cookie(&remote_address);

            StoppedClock::local_add(&COOKIE_LIFETIME.increment).unwrap();

            TestConnectionCookie::check_connection_cookie(&remote_address, &cookie).unwrap();
        }

        #[test]
        fn it_should_be_valid_for_the_last_time_extent() {
            let remote_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);

            let cookie = TestConnectionCookie::make_connection_cookie(&remote_address);

            StoppedClock::local_set(&COOKIE_LIFETIME.total().unwrap().unwrap());

            TestConnectionCookie::check_connection_cookie(&remote_address, &cookie).unwrap();
        }

        #[test]
        #[should_panic]
        fn it_should_be_not_valid_after_their_last_time_extent() {
            let remote_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);

            let cookie = TestConnectionCookie::make_connection_cookie(&remote_address);

            StoppedClock::local_set(&COOKIE_LIFETIME.total_next().unwrap().unwrap());

            TestConnectionCookie::check_connection_cookie(&remote_address, &cookie).unwrap();
        }

        mod detail {
            use std::net::{IpAddr, Ipv4Addr, SocketAddr};

            use crate::udp::connection_cookie::WitnessConnectionCookie;

            #[test]
            fn it_should_build_the_same_connection_cookie_for_the_same_input_data() {
                let remote_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);
                let expiry_zero = [0u8; 4];

                let cookie = WitnessConnectionCookie::build(&remote_address, expiry_zero);
                let cookie_2 = WitnessConnectionCookie::build(&remote_address, expiry_zero);

                // witness is the same, and the times are the same.
                assert_eq!(cookie[0..4], cookie_2[0..4]);
                assert_eq!(cookie[5..8], cookie_2[5..8]);
            }

            #[test]
            fn it_should_build_the_different_connection_cookie_for_different_ip() {
                let remote_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);
                let remote_address_2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::BROADCAST), 0);
                let expiry_zero = [0u8; 4];

                let cookie = WitnessConnectionCookie::build(&remote_address, expiry_zero);
                let cookie_2 = WitnessConnectionCookie::build(&remote_address_2, expiry_zero);

                // witness is different, but the times are the same.
                assert_ne!(cookie[0..4], cookie_2[0..4]);
                assert_eq!(cookie[5..8], cookie_2[5..8]);
            }

            #[test]
            fn it_should_build_the_different_connection_cookie_for_different_expires() {
                let remote_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);
                let expiry_zero = [0u8; 4];
                let expiry_max = [255u8; 4];

                let cookie = WitnessConnectionCookie::build(&remote_address, expiry_zero);
                let cookie_2 = WitnessConnectionCookie::build(&remote_address, expiry_max);

                // witness is different, and the times are different.
                assert_ne!(cookie[0..4], cookie_2[0..4]);
                assert_ne!(cookie[5..8], cookie_2[5..8]);
            }
        }
    }

    mod encrypted_connection_cookie {
        use std::net::{IpAddr, Ipv4Addr, SocketAddr};

        use crate::protocol::clock::time_extent::Extent;
        use crate::protocol::clock::{StoppedClock, StoppedTime};
        use crate::udp::connection_cookie::{
            ConnectionCookie, Cookie, EncryptedConnectionCookie as TestConnectionCookie, COOKIE_LIFETIME,
        };

        #[test]
        fn it_should_make_a_connection_cookie() {
            let remote_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);
            let cookie = TestConnectionCookie::make_connection_cookie(&remote_address);

            // Note: This constant may need to be updated in the future as the hash is not guaranteed to to be stable between versions.
            const ID_COOKIE: Cookie = [234, 229, 134, 15, 217, 77, 208, 204];

            assert_eq!(cookie, ID_COOKIE)
        }

        #[test]
        fn it_should_make_different_cookies_for_the_next_time_extent() {
            let remote_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);

            let cookie = TestConnectionCookie::make_connection_cookie(&remote_address);

            StoppedClock::local_add(&COOKIE_LIFETIME.increment).unwrap();

            let cookie_next = TestConnectionCookie::make_connection_cookie(&remote_address);

            assert_ne!(cookie, cookie_next)
        }

        #[test]
        fn it_should_be_valid_for_this_time_extent() {
            let remote_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);

            let cookie = TestConnectionCookie::make_connection_cookie(&remote_address);

            TestConnectionCookie::check_connection_cookie(&remote_address, &cookie).unwrap();
        }

        #[test]
        fn it_should_be_valid_for_the_next_time_extent() {
            let remote_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);

            let cookie = TestConnectionCookie::make_connection_cookie(&remote_address);

            StoppedClock::local_add(&COOKIE_LIFETIME.increment).unwrap();

            TestConnectionCookie::check_connection_cookie(&remote_address, &cookie).unwrap();
        }

        #[test]
        fn it_should_be_valid_for_the_last_time_extent() {
            let remote_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);

            let cookie = TestConnectionCookie::make_connection_cookie(&remote_address);

            StoppedClock::local_set(&COOKIE_LIFETIME.total().unwrap().unwrap());

            TestConnectionCookie::check_connection_cookie(&remote_address, &cookie).unwrap();
        }

        #[test]
        #[should_panic]
        fn it_should_be_not_valid_after_their_last_time_extent() {
            let remote_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);

            let cookie = TestConnectionCookie::make_connection_cookie(&remote_address);

            StoppedClock::local_set(&COOKIE_LIFETIME.total_next().unwrap().unwrap());

            TestConnectionCookie::check_connection_cookie(&remote_address, &cookie).unwrap();
        }
    }
}
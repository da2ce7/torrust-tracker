use std::collections::hash_map::DefaultHasher;
use std::convert::TryInto;
use std::hash::Hasher;
use std::time::Duration;

use blowfish::Blowfish;
use byteorder::{LittleEndian, LE};
use cipher::generic_array::GenericArray;
use cipher::{BlockDecrypt, BlockEncrypt, BlockSizeUser};

use crate::keys::DEFAULT_KEY;
use crate::protocol::clock::{DefaultTime, Time};

type BlowfishArray = GenericArray<u8, <Blowfish<LittleEndian> as BlockSizeUser>::BlockSize>;

use super::client_image::{ClientImage, KeyedImage, PlainImage};

pub trait ConnectionCookie<T: ClientImage> {
    type Error;

    fn new(client_image: T, lifetime: Duration) -> Self;
    fn check(client_image: T, lifetime: Duration, encoded_id: [u8; 8], after_now: Option<Duration>) -> Result<(), Self::Error>;

    fn value(&self) -> &[u8; 8];
}

#[derive(PartialEq, Debug)]
pub struct HashedCookie {
    value: [u8; 8],
}

#[derive(PartialEq, Debug)]
pub struct WitnessCookie {
    value: [u8; 8],
}

#[derive(PartialEq, Debug)]
pub struct EncryptedCookie {
    value: [u8; 8],
}

impl HashedCookie {
    fn new_int(client_image: KeyedImage, lifetime: Duration, past: bool, after_now: Option<Duration>) -> Self {
        
        let after_maybe = match after_now {
            Some(after) => after,
            None => Duration::ZERO,
        };
        
        let now_maybe_next = match past {
            true => DefaultTime::after(&after_maybe),
            false => DefaultTime::after(&(lifetime + after_maybe)),
        };

        let life_period =  now_maybe_next / lifetime.into();

        let mut hasher = DefaultHasher::default();
        hasher.write_u128(life_period);
        hasher.write(client_image.value().as_slice());

        HashedCookie {
            value: hasher.finish().to_le_bytes(),
        }
    }
}

impl ConnectionCookie<KeyedImage> for HashedCookie {
    type Error = &'static str;

    fn new(client_image: KeyedImage, lifetime: Duration) -> Self {
        Self::new_int(client_image, lifetime, false, None)
    }

    fn value(&self) -> &[u8; 8] {
        &self.value
    }

    fn check(
        client_image: KeyedImage,
        lifetime: Duration,
        encoded_id: [u8; 8],
        after_now: Option<Duration>,
    ) -> Result<(), Self::Error> {
        if Self::new_int(client_image.clone(), lifetime, false, after_now).value == encoded_id {
            println!("Now");
            return Ok(());
        } else if Self::new_int(client_image.clone(), lifetime, true, after_now).value == encoded_id {
            println!("Past");
            return Ok(());
        } else {
            return Err("Expired connection id");
        }
    }
}

impl ConnectionCookie<KeyedImage> for WitnessCookie {
    type Error = &'static str;

    fn new(client_image: KeyedImage, lifetime: Duration) -> Self {
        let expiry_u32: u32 = DefaultTime::after(&lifetime).try_into().unwrap();

        let mut hasher = DefaultHasher::default();
        hasher.write_u32(expiry_u32);
        hasher.write(client_image.value().as_slice());
        let witness = hasher.finish().to_le_bytes();

        let id: Vec<u8> = [&witness[0..4], expiry_u32.to_le_bytes().as_slice()].concat();

        WitnessCookie {
            value: id.try_into().unwrap(),
        }
    }

    fn value(&self) -> &[u8; 8] {
        &self.value
    }

    fn check(
        client_image: KeyedImage,
        _lifetime: Duration,
        encoded_id: [u8; 8],
        after_now: Option<Duration>,
    ) -> Result<(), Self::Error> {
        let expiry_bytes: [u8; 4] = encoded_id[4..8].try_into().unwrap();
        let expiry_u32 = u32::from_le_bytes(expiry_bytes);

        let after_maybe = match after_now {
            Some(after) => after,
            None => Duration::ZERO,
        };

        let now_u32 = DefaultTime::after(&after_maybe).try_into().unwrap();

        if expiry_u32 <= now_u32 {
            return Err("Expired connection id");
        }

        let mut hasher = DefaultHasher::default();
        hasher.write_u32(expiry_u32);
        hasher.write(client_image.value().as_slice());

        let witness = hasher.finish().to_le_bytes();

        if &witness[0..4] != &encoded_id[0..4] {
            Err("Bad Witness in Connection Id")
        } else {
            Ok(())
        }
    }
}

impl ConnectionCookie<PlainImage> for EncryptedCookie {
    type Error = &'static str;

    fn new(client_image: PlainImage, lifetime: Duration) -> Self {
        let expiry_u32: u32 = DefaultTime::after(&lifetime).try_into().unwrap();

        let id_clear: Vec<u8> = [&client_image.value()[0..4], &expiry_u32.to_le_bytes().as_slice()].concat();

        let id_clear_bytes: [u8; 8] = id_clear.try_into().unwrap();
        let mut block: BlowfishArray = BlowfishArray::from(id_clear_bytes);

        let blowfish_key: &blowfish::Blowfish<LE> = &DEFAULT_KEY;

        <Blowfish<LE> as BlockEncrypt>::encrypt_block(blowfish_key, &mut block);

        EncryptedCookie {
            value: block.try_into().unwrap(),
        }
    }

    fn value(&self) -> &[u8; 8] {
        &self.value
    }

    fn check(
        client_image: PlainImage,
        _lifetime: Duration,
        encoded_id: [u8; 8],
        after_now: Option<Duration>,
    ) -> Result<(), Self::Error> {
        let mut block: BlowfishArray = BlowfishArray::from(encoded_id);

        let blowfish_key: &blowfish::Blowfish<LE> = &DEFAULT_KEY;

        <Blowfish<LE> as BlockDecrypt>::decrypt_block(&blowfish_key, &mut block);

        let client_image_bytes: [u8; 4] = block[0..4].try_into().unwrap();
        let expiry_bytes: [u8; 4] = block[4..8].try_into().unwrap();

        let after_maybe = match after_now {
            Some(after) => after,
            None => Duration::ZERO,
        };

        let now = DefaultTime::after(&after_maybe);

        let expiry = u32::from_le_bytes(expiry_bytes);

        if expiry <= now.try_into().unwrap() {
            return Err("Expired connection id");
        }

        if &client_image_bytes != &client_image.value()[0..4] {
            Err("Bad Witness in Connection Id")
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use std::time::Duration;

    use crate::udp::connection::client_image::Create;
    use crate::udp::connection::connection_cookie::ConnectionCookie;

    static LIFETIME_SEC : u64 = 120;

    mod test_hashed_encoded_id {
        use super::*;
        use crate::udp::connection::client_image::KeyedImage as Image;
        use crate::udp::connection::connection_cookie::HashedCookie as Id;

        #[test]
        fn it_should_make_an_encoded_id() {
            let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
            let client_image = Image::new(&socket);
            let lifetime = Duration::new(LIFETIME_SEC, 0);
            let encoded_id = Id::new(client_image, lifetime);

            assert_eq!(encoded_id.value, [61, 111, 96, 9, 22, 1, 242, 62]);
        }

        #[test]
        fn it_should_check_an_encoded_id() {
            let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
            let client_image = Image::new(&socket);
            let lifetime = Duration::new(LIFETIME_SEC, 0);
            let encoded_id = Id::new(client_image.to_owned(), lifetime);
            let result = Id::check(client_image.to_owned(), lifetime, encoded_id.value, None);

            assert_eq!(result, Ok(()));
        }

        #[test]
        fn it_should_check_an_expired_encoded_id() {
            let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
            let client_image = Image::new(&socket);
            let lifetime = Duration::new(LIFETIME_SEC, 0);
            let encoded_id = Id::new(client_image.to_owned(), lifetime);

            let result = Id::check(client_image.to_owned(), lifetime, encoded_id.value, Some(lifetime + lifetime));

            assert_ne!(result, Ok(()));
        }
    }

    mod test_witness_encoded_id {
        use super::*;
        use crate::udp::connection::client_image::KeyedImage as Image;
        use crate::udp::connection::connection_cookie::WitnessCookie as Id;

        #[test]
        fn it_should_make_an_encoded_id() {
            let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
            let client_image = Image::new(&socket);
            let lifetime = Duration::new(LIFETIME_SEC, 0);
            let encoded_id = Id::new(client_image, lifetime);

            assert_eq!(encoded_id.value, [84, 184, 248, 3, 120, 0, 0, 0]);
        }

        #[test]
        fn it_should_check_an_encoded_id() {
            let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
            let client_image = Image::new(&socket);
            let lifetime = Duration::new(LIFETIME_SEC, 0);
            let encoded_id = Id::new(client_image.to_owned(), lifetime);
            let result = Id::check(client_image.to_owned(), lifetime, encoded_id.value, None);

            assert_eq!(result, Ok(()));
        }

        #[test]
        fn it_should_check_an_expired_encoded_id() {
            let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
            let client_image = Image::new(&socket);
            let lifetime = Duration::new(LIFETIME_SEC, 0);
            let encoded_id = Id::new(client_image.to_owned(), lifetime);
            let result = Id::check(client_image.to_owned(), lifetime, encoded_id.value, Some(lifetime));

            assert_ne!(result, Ok(()));
        }
    }

    mod test_encrypted_encoded_id {
        use super::*;
        use crate::udp::connection::client_image::{PlainImage as Image, PlainHash};
        use crate::udp::connection::connection_cookie::EncryptedCookie as Id;

        #[test]
        fn it_should_make_an_encoded_id() {
            let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
            let client_image = <Image as Create<PlainHash>>::new(&socket);
            let lifetime = Duration::new(LIFETIME_SEC, 0);
            let encoded_id = Id::new(client_image, lifetime);

            let id_value_one = [214, 164, 226, 234, 70, 109, 133, 61];
            let id_value_two = [183, 115, 182, 14, 70, 205, 150, 31];

            if encoded_id.value == id_value_one {
            } else if encoded_id.value == id_value_two {
            } else {
                assert!(
                    false,
                    "encoded_id.value: {:?}, does not match, {id_value_one:?} or {id_value_two:?}",
                    encoded_id.value
                );
            }
        }

        #[test]
        fn it_should_check_an_encoded_id() {
            let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
            let client_image = <Image as Create<PlainHash>>::new(&socket);
            let lifetime = Duration::new(LIFETIME_SEC, 0);
            let encoded_id = Id::new(client_image.to_owned(), lifetime);
            let result = Id::check(client_image.to_owned(), lifetime, encoded_id.value, None);

            assert_eq!(result, Ok(()));
        }

        #[test]
        fn it_should_check_an_expired_encoded_id() {
            let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
            let client_image = <Image as Create<PlainHash>>::new(&socket);
            let lifetime = Duration::new(LIFETIME_SEC, 0);
            let encoded_id = Id::new(client_image.to_owned(), lifetime);
            let result = Id::check(client_image.to_owned(), lifetime, encoded_id.value, Some(lifetime));

            assert_ne!(result, Ok(()));
        }
    }
}

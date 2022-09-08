//! ClientImage is a unique image of the UDP tracker client.
//! Currently implemented with a hash of the socket, i.e the IP and port.
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::net::SocketAddr;

use crate::keys::DEFAULT_SECRET;

pub trait ClientImage: PartialEq + Clone {
    fn value(&self) -> &Vec<u8>;
}

#[derive(PartialEq, Debug, Clone)]
pub struct PlainImage {
    value: Vec<u8>,
}

#[derive(PartialEq, Debug, Clone)]
pub struct KeyedImage {
    value: Vec<u8>,
}

impl ClientImage for PlainImage {
    fn value(&self) -> &Vec<u8> {
        &self.value
    }
}
impl ClientImage for KeyedImage {
    fn value(&self) -> &Vec<u8> {
        &self.value
    }
}

pub struct KeyedHash;
pub struct PlainHash;

pub trait Create<T>: Digest<T> {
    fn new(socket: &SocketAddr) -> Self;
}

pub trait Digest<T> {
    fn hash(socket: &SocketAddr) -> Vec<u8>;
}

impl<T: Default + Hasher, U> Digest<T> for U {
    fn hash(socket: &SocketAddr) -> Vec<u8> {
        let mut hasher = T::default();
        socket.hash(&mut hasher);

        hasher.finish().to_le_bytes().to_vec()
    }
}

impl<U> Digest<PlainHash> for U {
    fn hash(socket: &SocketAddr) -> Vec<u8> {
        <PlainHash as Digest<DefaultHasher>>::hash(&socket)
    }
}

impl<U> Digest<KeyedHash> for U {
    fn hash(socket: &SocketAddr) -> Vec<u8> {
        let secret_key: [u8; 32] = *DEFAULT_SECRET;

        blake3::keyed_hash(&secret_key, &socket.to_string().as_bytes())
            .as_bytes()
            .to_vec()
    }
}

impl Create<PlainHash> for PlainImage {
    fn new(socket: &SocketAddr) -> Self {
        PlainImage {
            value: <PlainImage as Digest<PlainHash>>::hash(socket),
        }
    }
}

impl Create<KeyedHash> for KeyedImage {
    fn new(socket: &SocketAddr) -> Self {
        KeyedImage {
            value: <KeyedImage as Digest<KeyedHash>>::hash(socket),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    use super::Create;

    mod test_plain_client_image {
        use super::super::PlainHash as Digest;
        use super::super::PlainImage as Image;
        use super::*;

        #[test]
        fn it_should_be_a_hash_of_the_socket() {
            let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
            let image: Image = Create::<Digest>::new(&socket);

            let image_value_one = [43, 97, 213, 79, 136, 159, 106, 60];
            let image_value_two = [213, 195, 130, 185, 196, 163, 197, 161];

            if image.value == image_value_one {
            } else if image.value == image_value_two {
            } else {
                assert!(
                    false,
                    "image.value: {:?}, does not match, {image_value_one:?} or {image_value_two:?}",
                    image.value
                );
            }
        }

        #[test]
        fn it_should_be_unique_with_different_socket_ips() {
            let socket_1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
            let socket_2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2)), 8080);

            assert_ne!(
                <Image as Create::<Digest>>::new(&socket_1),
                <Image as Create::<Digest>>::new(&socket_2)
            );
        }

        #[test]
        fn it_should_be_unique_with_different_socket_ports() {
            let socket_1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
            let socket_2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8081);

            assert_ne!(
                <Image as Create::<Digest>>::new(&socket_1),
                <Image as Create::<Digest>>::new(&socket_2)
            );
        }
    }

    mod test_keyed_client_image {
        use super::super::{KeyedHash as Digest, KeyedImage as Image};
        use super::*;

        #[test]
        fn it_should_be_a_hash_of_the_socket() {
            let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
            let image: Image = Create::<Digest>::new(&socket);

            assert_eq!(
                image.value,
                [157, 254, 130, 232, 125, 218, 128, 209, 20, 63, 24, 126, 87, 200, 43, 98, 140, 53, 179, 63, 39, 54, 49, 12, 199, 101, 69, 52, 127, 148, 175, 231]
            );
        }

        #[test]
        fn it_should_be_unique_with_different_socket_ips() {
            let socket_1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
            let socket_2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2)), 8080);

            assert_ne!(
                <Image as Create::<Digest>>::new(&socket_1),
                <Image as Create::<Digest>>::new(&socket_2)
            );
        }

        #[test]
        fn it_should_be_unique_with_different_socket_ports() {
            let socket_1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
            let socket_2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8081);

            assert_ne!(
                <Image as Create::<Digest>>::new(&socket_1),
                <Image as Create::<Digest>>::new(&socket_2)
            );
        }
    }
}

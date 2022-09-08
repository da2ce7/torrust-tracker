use aquatic_udp_protocol::ConnectionId;
use criterion::{criterion_group, criterion_main, Criterion};

use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};
use std::time::Duration;

use torrust_tracker::protocol::clock::{DefaultClock, Clock};
use torrust_tracker::udp::connection::client_image::{Create, KeyedImage, PlainImage, PlainHash};
use torrust_tracker::udp::connection::connection_cookie::{
    ConnectionCookie, EncryptedCookie, HashedCookie, WitnessCookie,
};

fn get_connection_id_old(current_time: u64, port: u16) -> ConnectionId {
    let time_i64 = (current_time / 3600) as i64;

    ConnectionId((time_i64 | port as i64) << 36)
}

pub fn benchmark_make_old_unencoded_id(bench: &mut Criterion) {
    let remote_address = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 117);
    let current_time = DefaultClock::now();

    bench.bench_function("benchmark_make_old_unencoded_id", |b| {
        b.iter(|| {
            // Inner closure, the actual test
            let _ = get_connection_id_old(current_time.into(), remote_address.port());
        })
    });
}

pub fn benchmark_make_hashed_encoded_id(bench: &mut Criterion) {
    let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

    bench.bench_function("benchmark_make_hashed_encoded_id", |b| {
        b.iter(|| {
            // Inner closure, the actual test
            let client_image = KeyedImage::new(&socket);
            let _ = HashedCookie::new(client_image, Duration::new(1, 0));
        })
    });
}

pub fn benchmark_make_witness_encoded_id(bench: &mut Criterion) {
    let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

    bench.bench_function("benchmark_make_witness_encoded_id", |b| {
        b.iter(|| {
            // Inner closure, the actual test
            let client_image = KeyedImage::new(&socket);
            let _ = WitnessCookie::new(client_image, Duration::new(1, 0));
        })
    });
}

pub fn benchmark_make_encrypted_encoded_id(bench: &mut Criterion) {
    let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

    bench.bench_function("benchmark_make_encrypted_encoded_id", |b| {
        b.iter(|| {
            // Inner closure, the actual test
            let client_image = <PlainImage as Create<PlainHash>>::new(&socket);
            let _ = EncryptedCookie::new(client_image, Duration::new(1, 0));
        })
    });
}

criterion_group!(
    benches,
    benchmark_make_old_unencoded_id,
    benchmark_make_hashed_encoded_id,
    benchmark_make_witness_encoded_id,
    benchmark_make_encrypted_encoded_id,
);
criterion_main!(benches);

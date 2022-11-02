/// Integration tests for UDP tracker server
///
/// cargo test udp_tracker_server -- --nocapture
extern crate rand;

mod udp_tracker_server {
    use core::panic;
    use std::io::Cursor;
    use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    use aquatic_udp_protocol::{
        AnnounceEvent, AnnounceRequest, ConnectRequest, ConnectionId, InfoHash, NumberOfBytes, NumberOfPeers, PeerId, PeerKey,
        Port, Request, Response, ScrapeRequest, TransactionId,
    };
    use rand::{thread_rng, Rng};
    use tokio::net::UdpSocket;
    use tokio::task::JoinHandle;
    use torrust_tracker::jobs::udp_tracker;
    use torrust_tracker::tracker::helpers::TrackerArgs;
    use torrust_tracker::tracker::mode::TrackerMode;
    use torrust_tracker::tracker::tracker::TorrentTracker;
    use torrust_tracker::udp::{UdpServiceSettings, MAX_PACKET_SIZE};
    use torrust_tracker::{logging, static_time};

    pub struct UdpTestSettings {
        tracker: TrackerArgs,
        service: UdpServiceSettings,
    }

    impl Default for UdpTestSettings {
        fn default() -> Self {
            Self {
                tracker: TrackerArgs::new().no_logs().mode(TrackerMode::Public),
                service: UdpServiceSettings {
                    id: "test".to_string(),
                    display_name: "UDP Test Service".to_string(),
                    socket: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, ephemeral_random_port())),
                },
            }
        }
    }

    fn tracker_settings() -> Arc<UdpTestSettings> {
        Arc::new(UdpTestSettings::default())
    }

    pub struct UdpServer {
        pub started: AtomicBool,
        pub job: Option<JoinHandle<()>>,
        pub socket: Option<SocketAddr>,
    }

    impl UdpServer {
        pub fn new() -> Self {
            Self {
                started: AtomicBool::new(false),
                job: None,
                socket: None,
            }
        }

        pub async fn start(&mut self, settings: Arc<UdpTestSettings>) {
            if !self.started.load(Ordering::Relaxed) {
                // Set the time of Torrust app starting
                lazy_static::initialize(&static_time::TIME_AT_APP_START);

                // Initialize Torrust tracker
                let tracker = match TorrentTracker::new(
                    &settings.tracker.global,
                    &settings.tracker.common,
                    (settings.tracker.stats_tracker)(),
                    (settings.tracker.database)(),
                ) {
                    Ok(tracker) => Arc::new(tracker),
                    Err(error) => {
                        panic!("{}", error)
                    }
                };

                // Initialize logging
                logging::setup_logging(&settings.tracker.global);

                // Start the UDP tracker job
                self.job = Some(udp_tracker::start_job(&settings.service, tracker.clone()));

                self.socket = Some(settings.service.socket);

                self.started.store(true, Ordering::Relaxed);
            }
        }
    }

    async fn new_running_udp_server(settings: Arc<UdpTestSettings>) -> UdpServer {
        let mut udp_server = UdpServer::new();
        udp_server.start(settings).await;
        udp_server
    }

    struct UdpClient {
        socket: Arc<UdpSocket>,
    }

    impl UdpClient {
        async fn bind(local_address: &str) -> Self {
            let socket = UdpSocket::bind(local_address).await.unwrap();
            Self {
                socket: Arc::new(socket),
            }
        }

        async fn connect(&self, remote_socket: &SocketAddr) {
            self.socket.connect(remote_socket).await.unwrap()
        }

        async fn send(&self, bytes: &[u8]) -> usize {
            self.socket.writable().await.unwrap();
            self.socket.send(bytes).await.unwrap()
        }

        async fn receive(&self, bytes: &mut [u8]) -> usize {
            self.socket.readable().await.unwrap();
            self.socket.recv(bytes).await.unwrap()
        }
    }

    /// Creates a new UdpClient connected to a Udp server
    async fn new_connected_udp_client(remote_socket: &SocketAddr) -> UdpClient {
        let client = UdpClient::bind(&source_address(ephemeral_random_port())).await;
        client.connect(remote_socket).await;
        client
    }

    struct UdpTrackerClient {
        pub udp_client: UdpClient,
    }

    impl UdpTrackerClient {
        async fn send(&self, request: Request) -> usize {
            // Write request into a buffer
            let request_buffer = vec![0u8; MAX_PACKET_SIZE];
            let mut cursor = Cursor::new(request_buffer);

            let request_data = match request.write(&mut cursor) {
                Ok(_) => {
                    let position = cursor.position() as usize;
                    let inner_request_buffer = cursor.get_ref();
                    // Return slice which contains written request data
                    &inner_request_buffer[..position]
                }
                Err(_) => panic!("could not write request to bytes."),
            };

            self.udp_client.send(&request_data).await
        }

        async fn receive(&self) -> Response {
            let mut response_buffer = [0u8; MAX_PACKET_SIZE];

            let payload_size = self.udp_client.receive(&mut response_buffer).await;

            Response::from_bytes(&response_buffer[..payload_size], true).unwrap()
        }
    }

    /// Creates a new UdpTrackerClient connected to a Udp Tracker server
    async fn new_connected_udp_tracker_client(remote_socket: &SocketAddr) -> UdpTrackerClient {
        let udp_client = new_connected_udp_client(remote_socket).await;
        UdpTrackerClient { udp_client }
    }

    fn empty_udp_request() -> [u8; MAX_PACKET_SIZE] {
        [0; MAX_PACKET_SIZE]
    }

    fn empty_buffer() -> [u8; MAX_PACKET_SIZE] {
        [0; MAX_PACKET_SIZE]
    }

    /// Generates a random ephemeral port for a client source address
    fn ephemeral_random_port() -> u16 {
        // todo: this may produce random test failures because two tests can try to bind the same port.
        // We could either use the same client for all tests (slower) or
        // create a pool of available ports (with read/write lock)
        let mut rng = thread_rng();
        rng.gen_range(49152..65535)
    }

    /// Generates the source address for the UDP client
    fn source_address(port: u16) -> String {
        format!("127.0.0.1:{}", port)
    }

    fn is_error_response(response: &Response, error_message: &str) -> bool {
        match response {
            Response::Error(error_response) => return error_response.message.starts_with(error_message),
            _ => return false,
        };
    }

    fn is_connect_response(response: &Response, transaction_id: TransactionId) -> bool {
        match response {
            Response::Connect(connect_response) => return connect_response.transaction_id == transaction_id,
            _ => return false,
        };
    }

    fn is_ipv4_announce_response(response: &Response) -> bool {
        match response {
            Response::AnnounceIpv4(_) => return true,
            _ => return false,
        };
    }

    fn is_scrape_response(response: &Response) -> bool {
        match response {
            Response::Scrape(_) => return true,
            _ => return false,
        };
    }

    #[tokio::test]
    async fn should_return_a_bad_request_response_when_the_client_sends_an_empty_request() {
        let settings = tracker_settings();

        let udp_server = new_running_udp_server(settings).await;

        let client = new_connected_udp_client(&udp_server.socket.unwrap()).await;

        client.send(&empty_udp_request()).await;

        let mut buffer = empty_buffer();
        client.receive(&mut buffer).await;
        let response = Response::from_bytes(&buffer, true).unwrap();

        assert!(is_error_response(&response, "bad request"));
    }

    #[tokio::test]
    async fn should_return_a_connect_response_when_the_client_sends_a_connection_request() {
        let settings = tracker_settings();

        let udp_server = new_running_udp_server(settings).await;

        let client = new_connected_udp_tracker_client(&udp_server.socket.unwrap()).await;

        let connect_request = ConnectRequest {
            transaction_id: TransactionId(123),
        };

        client.send(connect_request.into()).await;

        let response = client.receive().await;

        assert!(is_connect_response(&response, TransactionId(123)));
    }

    async fn send_connection_request(transaction_id: TransactionId, client: &UdpTrackerClient) -> ConnectionId {
        let connect_request = ConnectRequest { transaction_id };

        client.send(connect_request.into()).await;

        let response = client.receive().await;

        match response {
            Response::Connect(connect_response) => connect_response.connection_id,
            _ => panic!("error connecting to udp server {:?}", response),
        }
    }

    #[tokio::test]
    async fn should_return_an_announce_response_when_the_client_sends_an_announce_request() {
        let settings = tracker_settings();

        let udp_server = new_running_udp_server(settings).await;

        let client = new_connected_udp_tracker_client(&udp_server.socket.unwrap()).await;

        let connection_id = send_connection_request(TransactionId(123), &client).await;

        // Send announce request

        let announce_request = AnnounceRequest {
            connection_id: ConnectionId(connection_id.0),
            transaction_id: TransactionId(123i32),
            info_hash: InfoHash([0u8; 20]),
            peer_id: PeerId([255u8; 20]),
            bytes_downloaded: NumberOfBytes(0i64),
            bytes_uploaded: NumberOfBytes(0i64),
            bytes_left: NumberOfBytes(0i64),
            event: AnnounceEvent::Started,
            ip_address: Some(Ipv4Addr::new(0, 0, 0, 0)),
            key: PeerKey(0u32),
            peers_wanted: NumberOfPeers(1i32),
            port: Port(client.udp_client.socket.local_addr().unwrap().port()),
        };

        client.send(announce_request.into()).await;

        let response = client.receive().await;

        eprintln!("{:?}", response);

        assert!(is_ipv4_announce_response(&response));
    }

    #[tokio::test]
    async fn should_return_a_scrape_response_when_the_client_sends_a_scrape_request() {
        let settings = tracker_settings();

        let udp_server = new_running_udp_server(settings).await;

        let client = new_connected_udp_tracker_client(&udp_server.socket.unwrap()).await;

        let connection_id = send_connection_request(TransactionId(123), &client).await;

        // Send scrape request

        // Full scrapes are not allowed so it will return "bad request" error with empty vector
        let info_hashes = vec![InfoHash([0u8; 20])];

        let scrape_request = ScrapeRequest {
            connection_id: ConnectionId(connection_id.0),
            transaction_id: TransactionId(123i32),
            info_hashes,
        };

        client.send(scrape_request.into()).await;

        let response = client.receive().await;

        assert!(is_scrape_response(&response));
    }
}

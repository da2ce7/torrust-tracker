use std::sync::Arc;

use futures::executor::block_on;
use torrust_tracker::bootstrap::app::initialize_with_configuration;
use torrust_tracker::bootstrap::jobs::make_rust_tls;
use torrust_tracker::core::Tracker;
use torrust_tracker::servers::http::server::{Server, Launcher, Running, Stopped};
use torrust_tracker::servers::registar::Registar;
use torrust_tracker_configuration::{Configuration, HttpTracker};
use torrust_tracker_primitives::info_hash::InfoHash;
use torrust_tracker_primitives::peer;

pub struct Environment<S> {
    pub config: Arc<HttpTracker>,
    pub tracker: Arc<Tracker>,
    pub registar: Registar,
    pub server: Server<S>,
}

impl<S> Environment<S> {
    /// Add a torrent to the tracker
    pub fn add_torrent_peer(&self, info_hash: &InfoHash, peer: &peer::Peer) {
        self.tracker.upsert_peer_and_get_stats(info_hash, peer);
    }
}

impl Environment<Stopped> {
    #[allow(dead_code)]
    pub fn new(configuration: &Arc<Configuration>) -> Self {
        let tracker = initialize_with_configuration(configuration);

        let http_tracker = configuration
            .http_trackers
            .clone()
            .expect("missing HTTP tracker configuration");

        let config = Arc::new(http_tracker[0].clone());

        let bind_to = config.bind_address;

        let tls = block_on(make_rust_tls(&config.tsl_config)).map(|tls| tls.expect("tls config failed"));

        let server = Server::new(Launcher::new(bind_to, tls));

        Self {
            config,
            tracker,
            registar: Registar::default(),
            server,
        }
    }

    #[allow(dead_code)]
    pub async fn start(self) -> Environment<Running> {
        Environment {
            config: self.config,
            tracker: self.tracker.clone(),
            registar: self.registar.clone(),
            server: self.server.start(self.tracker, self.registar.give_form()).await.unwrap(),
        }
    }
}

impl Environment<Running> {
    pub async fn new(configuration: &Arc<Configuration>) -> Self {
        Environment::<Stopped>::new(configuration).start().await
    }

    pub async fn stop(self) -> Environment<Stopped> {
        Environment {
            config: self.config,
            tracker: self.tracker,
            registar: Registar::default(),

            server: self.server.stop().await.unwrap(),
        }
    }

    pub fn bind_address(&self) -> &std::net::SocketAddr {
        &self.server.state.local_addr
    }
}

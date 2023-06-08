use std::{cmp::Ordering, collections::BTreeSet, fmt::Debug, net::SocketAddr, sync::Arc};

use crate::player_entry::PlayerArcWrapper;

#[derive(Debug, Clone)]
pub struct Server {
    pub addr: SocketAddr,
    pub players: BTreeSet<PlayerArcWrapper>,
}

impl Server {
    pub fn update(&mut self, other: &Server) {
        println!(
            "[Server] Merging self '{:?}' with other '{:?}'",
            self, other
        );
        let self_list = &mut self.players;
        for player in &other.players {
            let take = self_list.take(player);
            match take {
                Some(mut_player) => {
                    mut_player.lock().update(&player.lock());
                    self_list.insert(mut_player);
                }
                None => {
                    self_list.insert(player.clone());
                }
            }
        }
    }
}

impl PartialEq for Server {
    fn eq(&self, other: &Self) -> bool {
        self.addr == other.addr
    }
}

impl Eq for Server {}

impl PartialOrd for Server {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Server {
    fn cmp(&self, other: &Self) -> Ordering {
        self.addr.cmp(&other.addr)
    }
}

#[derive(Debug, Clone)]
pub struct ServerArcWrapper(Arc<parking_lot::Mutex<Server>>);

impl ServerArcWrapper {
    pub fn new(server: Server) -> Self {
        Self(Arc::new(parking_lot::Mutex::new(server)))
    }
    pub fn lock(&self) -> parking_lot::lock_api::MutexGuard<'_, parking_lot::RawMutex, Server> {
        self.0.lock()
    }
}

impl PartialOrd for ServerArcWrapper {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.lock().partial_cmp(&other.0.lock())
    }
}

impl Ord for ServerArcWrapper {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.lock().cmp(&other.0.lock())
    }
}

impl PartialEq for ServerArcWrapper {
    fn eq(&self, other: &Self) -> bool {
        self.0.lock().eq(&other.0.lock())
    }
}

impl Eq for ServerArcWrapper {}

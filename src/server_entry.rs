use std::{collections::BTreeSet, fmt::Debug, net::SocketAddr};

use crate::player_entry::PlayerArcWrapper;

#[derive(Debug, Clone)]
pub struct Server {
    pub addr: SocketAddr,
    pub players: BTreeSet<PlayerArcWrapper>,
}

#[allow(unused)]
impl Server {
    pub fn update(&mut self, other: &Server) {}
}

impl PartialEq for Server {
    fn eq(&self, other: &Self) -> bool {
        self.addr == other.addr
    }
}

use std::{collections::BTreeSet, fmt::Debug, net::SocketAddr};

use crate::player_entry::PlayerArcWrapper;

#[derive(Debug, Clone)]
pub struct Server {
    pub addr: SocketAddr,
    pub players: BTreeSet<PlayerArcWrapper>,
}

impl Server {
    #[allow(unused_variables)]
    pub fn update(&mut self, other: &Server) {
        println!("Merging self '{:?}' with other '{:?}'", self, other);
        let self_list = &mut self.players;
        for player in &other.players {
            let take = self_list.take(player);
            match take {
                Some(mut_player) => {
                    mut_player.lock().update(&player.lock());
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

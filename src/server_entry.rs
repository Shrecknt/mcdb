use std::{
    cmp::Ordering,
    collections::BTreeSet,
    error::Error,
    fmt::Debug,
    io::{Read, Write},
    net::SocketAddr,
    str::FromStr,
    sync::Arc,
};

use integer_encoding::{VarIntReader, VarIntWriter};

use crate::player_entry::PlayerArcWrapper;

#[derive(Debug, Clone)]
pub struct Server {
    pub addr: SocketAddr,
    pub players: BTreeSet<PlayerArcWrapper>,
}

impl Server {
    pub fn deserialize_pointer(mut buf: &[u8]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let bytes_size = buf.read_varint()?;
        let mut bytes = vec![0u8; bytes_size];
        buf.read_exact(&mut bytes)?;
        let addr_string = std::str::from_utf8(&bytes)?;
        let addr = SocketAddr::from_str(addr_string)?;
        let players: BTreeSet<PlayerArcWrapper> = BTreeSet::new();
        Ok(Server { addr, players })
    }

    pub fn serialize(&self) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        let mut res = vec![];
        let addr_string = self.addr.to_string();
        let addr_bytes = addr_string.as_bytes();
        res.write_varint(addr_bytes.len())?;
        res.write_all(addr_bytes)?;
        res.write_varint(self.players.len())?;
        let server_players = self.players.iter();
        for player in server_players {
            let player_bytes = player.lock().serialize_pointer()?;
            res.write_varint(player_bytes.len())?;
            res.write_all(&player_bytes)?;
        }
        Ok(res)
    }

    pub fn serialize_pointer(&self) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        let mut res = vec![];
        let addr_string = self.addr.to_string();
        let addr_bytes = addr_string.as_bytes();
        res.write_varint(addr_bytes.len())?;
        res.write_all(addr_bytes)?;
        Ok(res)
    }

    pub fn update(&mut self, other: &Server) {
        // println!(
        //     "[Server] Merging self '{:?}' with other '{:?}'",
        //     self, other
        // );
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

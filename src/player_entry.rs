use std::{
    cmp::Ordering,
    collections::BTreeSet,
    error::Error,
    io::{Read, Write},
    sync::Arc,
    vec,
};

use integer_encoding::{VarIntReader, VarIntWriter};
use uuid::Uuid;

use crate::server_entry::ServerArcWrapper;

#[derive(Debug)]
pub struct Player {
    pub name: String,
    pub uuid: Uuid,
    pub servers: BTreeSet<ServerArcWrapper>,
}

impl Player {
    pub fn deserialize(buf: &[u8]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let res = Self::deserialize_pointer(buf)?;
        Ok(res)
    }

    pub fn deserialize_pointer(mut buf: &[u8]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let name_len = buf.read_varint()?;
        let mut name = vec![0u8; name_len];
        buf.read_exact(&mut name)?;
        let name_string = std::str::from_utf8(&name);
        match name_string {
            Ok(name) => {
                let mut uuid_buf = [0u8; 16];
                buf.read_exact(&mut uuid_buf)?;
                let uuid = Uuid::from_bytes(uuid_buf);
                let servers: BTreeSet<ServerArcWrapper> = BTreeSet::new();
                Ok(Player {
                    name: name.to_string(),
                    uuid,
                    servers,
                })
            }
            Err(err) => Err(Box::new(err)),
        }
    }

    pub fn serialize(&self) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        let mut res = vec![];
        let uuid_bytes: &[u8; 16] = self.uuid.as_bytes();
        res.write_all(uuid_bytes)?;
        res.write_varint(self.servers.len())?;
        let player_servers = self.servers.iter();
        for server in player_servers {
            let server_bytes = server.lock().serialize_pointer()?;
            res.write_varint(server_bytes.len())?;
            res.write_all(&server_bytes)?;
        }
        Ok(res)
    }

    pub fn serialize_pointer(&self) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        let mut res = vec![];
        let name_bytes = self.name.as_bytes();
        res.write_varint(name_bytes.len())?;
        res.write_all(name_bytes)?;
        res.write_all(self.uuid.as_bytes())?;
        Ok(res)
    }

    pub fn update(&mut self, other: &Player) {
        // println!(
        //     "[Player] Merging self '{:?}' with other '{:?}'",
        //     self, other
        // );
        for server in &other.servers {
            let take = self.servers.take(server);
            match take {
                Some(mut_server) => {
                    mut_server.lock().update(&server.lock());
                    self.servers.insert(mut_server);
                }
                None => {
                    self.servers.insert(server.clone());
                }
            }
        }
    }
}

impl PartialOrd for Player {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let mut cmp = self.name.cmp(&other.name);
        if cmp == Ordering::Equal {
            cmp = self.uuid.cmp(&other.uuid);
        }
        Some(cmp)
    }
}

impl Ord for Player {
    fn cmp(&self, other: &Self) -> Ordering {
        let mut cmp = self.name.cmp(&other.name);
        if cmp == Ordering::Equal {
            cmp = self.uuid.cmp(&other.uuid);
        }
        cmp
    }
}

impl PartialEq for Player {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.uuid == other.uuid
    }
}

impl Eq for Player {}

impl Clone for Player {
    fn clone(&self) -> Self {
        Player {
            name: self.name.clone(),
            uuid: self.uuid,
            servers: BTreeSet::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlayerArcWrapper(Arc<parking_lot::Mutex<Player>>);

impl PlayerArcWrapper {
    pub fn new(player: Player) -> Self {
        Self(Arc::new(parking_lot::Mutex::new(player)))
    }
    pub fn lock(&self) -> parking_lot::lock_api::MutexGuard<'_, parking_lot::RawMutex, Player> {
        self.0.lock()
    }
}

impl PartialOrd for PlayerArcWrapper {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.lock().partial_cmp(&other.0.lock())
    }
}

impl Ord for PlayerArcWrapper {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.lock().cmp(&other.0.lock())
    }
}

impl PartialEq for PlayerArcWrapper {
    fn eq(&self, other: &Self) -> bool {
        self.0.lock().eq(&other.0.lock())
    }
}

impl Eq for PlayerArcWrapper {}

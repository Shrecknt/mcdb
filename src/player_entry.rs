use std::{cmp::Ordering, collections::BTreeSet, error::Error, sync::Arc, vec};

use integer_encoding::{VarIntAsyncReader, VarIntAsyncWriter};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;

use crate::server_entry::ServerArcWrapper;

#[derive(Debug)]
pub struct Player {
    pub name: String,
    pub uuid: Uuid,
    pub servers: BTreeSet<ServerArcWrapper>,
}

impl Player {
    pub async fn deserialize(buf: &[u8]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let res = Self::deserialize_pointer(buf).await?;
        Ok(res)
    }

    pub async fn deserialize_pointer(mut buf: &[u8]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let name_len = buf.read_varint_async().await?;
        let mut name = vec![0u8; name_len];
        buf.read_exact(&mut name).await?;
        let name_string = std::str::from_utf8(&name);
        match name_string {
            Ok(name) => {
                let uuid = Uuid::from_u128(buf.read_u128().await?);
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

    pub async fn serialize(&self) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        let mut res = vec![];
        let uuid_bytes: &[u8; 16] = self.uuid.as_bytes();
        res.write_all(uuid_bytes).await?;
        Ok(res)
    }

    pub async fn serialize_pointer(&self) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        let mut res = vec![];
        let name_bytes = self.name.as_bytes();
        res.write_varint_async(name_bytes.len()).await?;
        res.write_all(name_bytes).await?;
        res.write_all(self.uuid.as_bytes()).await?;
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

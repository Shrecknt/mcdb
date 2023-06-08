use std::{cmp::Ordering, error::Error, sync::Arc, vec};

use parking_lot::Mutex;
use tokio::io::AsyncWriteExt;
use uuid::{uuid, Uuid};

use crate::server_entry::Server;

#[derive(Debug)]
pub struct Player {
    pub name: String,
    pub uuid: Uuid,
    pub servers: Vec<Arc<Mutex<Server>>>,
}

impl Player {
    pub async fn deserialize() -> Result<Self, Box<dyn Error>> {
        let name = String::from("");
        let uuid = uuid!("9eaf436b-43eb-47f1-a26c-44306e076dfa");
        let servers: Vec<Arc<Mutex<Server>>> = vec![];

        Ok(Player {
            name,
            uuid,
            servers,
        })
    }

    pub async fn serialize(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut res = vec![];
        let uuid_bytes: &[u8; 16] = self.uuid.as_bytes();
        res.write_all(uuid_bytes).await?;

        Ok(res)
    }

    #[allow(unused_variables)]
    pub fn update(&mut self, other: &Player) {}
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
            servers: vec![],
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

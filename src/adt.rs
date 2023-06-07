use std::collections::{BTreeSet, HashMap};
use std::error::Error;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use tokio::sync::Mutex;

use crate::player_entry::Player;
use crate::server_entry::Server;

const PRE_RESERVE: bool = false;

#[derive(Debug)]
pub struct ServerMap {
    #[allow(clippy::type_complexity)]
    server_array: HashMap<u16, Arc<Mutex<HashMap<u16, HashMap<u16, Arc<Mutex<Server>>>>>>>,
    player_array: BTreeSet<Player>,
}

impl ServerMap {
    pub fn new() -> Self {
        let mut alloc_hashmap = HashMap::new();
        alloc_hashmap.reserve(65536);
        ServerMap {
            server_array: alloc_hashmap,
            player_array: BTreeSet::new(),
        }
    }

    pub async fn insert(&mut self, server_arc: Arc<Mutex<Server>>) -> Result<(), Box<dyn Error>> {
        let server = server_arc.lock().await;
        let octets: [u8; 4] = match server.addr.ip() {
            IpAddr::V4(addr) => addr.octets(),
            _ => return Err("Not an IPv4 Address".into()),
        };

        let a = u8s_to_u16(octets[0], octets[1]);
        let b = u8s_to_u16(octets[2], octets[3]);

        let open = &mut self.server_array;
        open.entry(a).or_insert_with(|| {
            let mut alloc_hashmap = HashMap::new();
            if PRE_RESERVE {
                alloc_hashmap.reserve(65536);
            }
            Arc::new(Mutex::new(alloc_hashmap))
        });

        let open = open.get_mut(&a).unwrap().clone();
        let mut open = open.lock().await;
        open.entry(b).or_insert_with(HashMap::new);

        let open = open.get_mut(&b).unwrap();
        let find = open.get(&server.addr.port());
        if find.is_some() {
            find.unwrap().lock().await.update(&server);
        } else {
            open.insert(server.addr.port(), server_arc.clone());
        }

        for player in &server.players {
            let player = player.lock().unwrap().clone();
            // let found = self.player_array.get(&*player);
            // ^ does not work because return value of `get`
            // is immutable, and we need a mutable reference
            // to call .update() on it

            // this is inefficient
            // TODO: use a RefCell?
            let found = self.player_array.take(&player);
            let to_insert: Player = if found.is_some() {
                let mut found = found.unwrap();
                found.update(&player);
                let has_server = player_has_server(&found, &server).await?;
                if !has_server {
                    found.servers.push(server_arc.clone());
                }
                found
            } else {
                let mut res = player.clone();
                res.servers.push(server_arc.clone());
                res
            };

            self.player_array.insert(to_insert);
        }

        Ok(())
    }

    pub async fn find(
        &mut self,
        addr: SocketAddr,
    ) -> Result<Option<Arc<Mutex<Server>>>, Box<dyn Error>> {
        let octets: [u8; 4] = match addr.ip() {
            IpAddr::V4(addr) => addr.octets(),
            _ => return Err("Not an IPv4 Address".into()),
        };

        let a = u8s_to_u16(octets[0], octets[1]);
        let b = u8s_to_u16(octets[2], octets[3]);

        let open = &mut self.server_array;
        if !open.contains_key(&a) {
            return Ok(None);
        }
        let open = open.get_mut(&a).unwrap();
        let mut open = open.lock().await;
        if !open.contains_key(&b) {
            return Ok(None);
        }
        let open = open.get_mut(&b).unwrap();
        let find = open.get(&addr.port());

        match find {
            Some(find) => Ok(Some(find.clone())),
            None => Ok(None),
        }
    }

    pub fn size(&self) -> usize {
        self.server_array.len()
    }
}

impl Default for ServerMap {
    fn default() -> Self {
        Self::new()
    }
}

fn u8s_to_u16(a: u8, b: u8) -> u16 {
    ((a as u16) << 8) | b as u16
}

async fn player_has_server(player: &Player, server: &Server) -> Result<bool, Box<dyn Error>> {
    for player_server in &player.servers {
        if *player_server.lock().await == *server {
            return Ok(true);
        }
    }
    Ok(false)
}

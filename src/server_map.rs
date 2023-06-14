use std::collections::{BTreeSet, HashMap};
use std::error::Error;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use parking_lot::Mutex;

use crate::player_entry::Player;
use crate::server_entry::{Server, ServerArcWrapper};

const PRE_RESERVE: bool = false;

#[derive(Debug)]
pub struct ServerMap {
    #[allow(clippy::type_complexity)]
    pub server_array: HashMap<u16, Arc<Mutex<HashMap<u16, HashMap<u16, ServerArcWrapper>>>>>,
    pub player_array: BTreeSet<Player>,
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

    pub fn insert(
        &mut self,
        server_arc: ServerArcWrapper,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut server = server_arc.lock();
        let octets: [u8; 4] = match server.addr.ip() {
            IpAddr::V4(addr) => addr.octets(),
            _ => return Err("Not an IPv4 Address".into()),
        };

        let a = u8s_to_u16(octets[0], octets[1]);
        let b = u8s_to_u16(octets[2], octets[3]);

        let open1 = &mut self.server_array;
        open1.entry(a).or_insert_with(|| {
            let mut alloc_hashmap = HashMap::new();
            if PRE_RESERVE {
                alloc_hashmap.reserve(65536);
            }
            Arc::new(Mutex::new(alloc_hashmap))
        });

        let open2 = open1.get_mut(&a).unwrap().clone();
        let mut open3 = open2.lock();
        open3.entry(b).or_insert_with(HashMap::new);

        let open4 = open3.get_mut(&b).unwrap();
        let find = open4.get(&server.addr.port());
        let inserted_arc: ServerArcWrapper;
        match find {
            Some(find) => {
                inserted_arc = find.clone();
                let temp = inserted_arc;
                let mut temp_lock = temp.lock();
                temp_lock.update(&server);
                drop(temp_lock);
            }
            None => {
                inserted_arc = server_arc.clone();
                open4.insert(server.addr.port(), inserted_arc);
            }
        };

        let server_players = server.players.clone();
        for player in server_players.iter() {
            let player = player.lock().clone();
            // let found = self.player_array.get(&*player);
            // ^ does not work because return value of `get`
            // is immutable, and we need a mutable reference
            // to call .update() on it

            // this is inefficient
            // TODO: use a RefCell?
            let found = self.player_array.take(&player);
            let to_insert: Player = match found {
                Some(mut found) => {
                    found.update(&player);
                    let has_server = player_has_server(&found, &server)?;
                    if !has_server {
                        drop(server);
                        // error here
                        found.servers.insert(server_arc.clone());
                        server = server_arc.lock();
                    }
                    found
                }
                None => {
                    let mut res = player.clone();
                    res.servers.insert(server_arc.clone());
                    res
                }
            };

            self.player_array.insert(to_insert);
        }

        Ok(())
    }

    pub fn find(
        &mut self,
        addr: SocketAddr,
    ) -> Result<Option<ServerArcWrapper>, Box<dyn Error + Send + Sync>> {
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
        let mut open = open.lock();
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

fn player_has_server(
    player: &Player,
    server: &Server,
) -> Result<bool, Box<dyn Error + Send + Sync>> {
    for player_server in &player.servers {
        if *player_server.lock() == *server {
            return Ok(true);
        }
    }
    Ok(false)
}

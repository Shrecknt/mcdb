pub mod adt;
pub mod player_entry;
pub mod server_entry;

use adt::ServerMap;
use player_entry::Player;
use server_entry::Server;

use std::collections::BTreeSet;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::{error::Error, sync::Arc};

use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::spawn;
use tokio::sync::Mutex;

use uuid::uuid;

use crate::player_entry::PlayerArcWrapper;

async fn handle_connection(
    socket: &mut TcpStream,
    map: Arc<Mutex<ServerMap>>,
) -> Result<(), Box<dyn Error>> {
    loop {
        let mut buf = [0u8; 6];
        let _ = socket.read_exact(&mut buf).await?;
        let mut push_bst: BTreeSet<PlayerArcWrapper> = BTreeSet::new();
        push_bst.insert(PlayerArcWrapper::new(Player {
            name: "test".to_string(),
            uuid: uuid!("F9168C5E-CEB2-4faa-B6BF-329BF39FA1E4"),
            servers: vec![],
        }));

        let mut lock = map.lock().await;

        lock.insert(Arc::new(Mutex::new(Server {
            addr: SocketAddr::V4(SocketAddrV4::new(
                Ipv4Addr::new(buf[0], buf[1], buf[2], buf[3]),
                u8s_to_u16(buf[4], buf[5]),
            )),
            players: push_bst,
        })))
        .await
        .unwrap();

        let found = lock
            .find(SocketAddr::V4(SocketAddrV4::new(
                Ipv4Addr::new(buf[0], buf[1], buf[2], buf[3]),
                u8s_to_u16(buf[4], buf[5]),
            )))
            .await?
            .unwrap();
        println!("found: {:?}", found.lock().await);

        drop(lock);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let map = Arc::new(Mutex::new(ServerMap::new()));

    let listener = TcpListener::bind("127.0.0.1:38282").await?;
    loop {
        let (mut socket, _) = listener.accept().await?;
        let clone_map = map.clone();
        spawn(async move { handle_connection(&mut socket, clone_map).await.unwrap() });
    }
}

fn u8s_to_u16(a: u8, b: u8) -> u16 {
    ((a as u16) << 8) | b as u16
}

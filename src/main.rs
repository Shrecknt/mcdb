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

use parking_lot::Mutex;

use uuid::uuid;

use crate::player_entry::PlayerArcWrapper;

async fn handle_connection(
    socket: &mut TcpStream,
    map: Arc<Mutex<ServerMap>>,
) -> Result<(), Box<dyn Error>> {
    loop {
        let mut buf = [0u8; 8];
        let _ = socket.read_exact(&mut buf).await?;
        let mut push_bst: BTreeSet<PlayerArcWrapper> = BTreeSet::new();
        let name = std::str::from_utf8(&buf[6..8]).unwrap();
        push_bst.insert(PlayerArcWrapper::new(Player {
            name: name.to_string(),
            uuid: uuid!("F9168C5E-CEB2-4faa-B6BF-329BF39FA1E4"),
            servers: vec![],
        }));

        let mut lock = map.lock();

        lock.insert(Arc::new(Mutex::new(Server {
            addr: SocketAddr::V4(SocketAddrV4::new(
                Ipv4Addr::new(buf[0], buf[1], buf[2], buf[3]),
                u8s_to_u16(buf[4], buf[5]),
            )),
            players: push_bst,
        })))
        .unwrap();

        let found = lock
            .find(SocketAddr::V4(SocketAddrV4::new(
                Ipv4Addr::new(buf[0], buf[1], buf[2], buf[3]),
                u8s_to_u16(buf[4], buf[5]),
            )))
            .unwrap()
            .unwrap();

        println!("found: {:?}", found.lock());

        println!(
            "found players: {:?}",
            found
                .lock()
                .players
                .iter()
                .map(|item| { item.lock().name.clone() })
                .collect::<Vec<String>>()
        );

        drop(lock);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    {
        use parking_lot::deadlock;
        use std::thread;
        use std::time::Duration;
        // Create a background thread which checks for deadlocks every 10s
        thread::spawn(move || loop {
            thread::sleep(Duration::from_secs(10));
            let deadlocks = deadlock::check_deadlock();
            if deadlocks.is_empty() {
                continue;
            }

            println!("{} deadlocks detected", deadlocks.len());
            for (i, threads) in deadlocks.iter().enumerate() {
                println!("Deadlock #{i}");
                for t in threads {
                    println!("Thread Id {:#?}", t.thread_id());
                    println!("{:#?}", t.backtrace());
                }
            }
        });
    }

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

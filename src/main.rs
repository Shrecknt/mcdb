pub mod player_entry;
pub mod server_entry;
pub mod server_map;

use player_entry::{Player, PlayerArcWrapper};
use server_entry::{Server, ServerArcWrapper};
use server_map::ServerMap;

use std::collections::BTreeSet;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::{error::Error, sync::Arc};

use integer_encoding::VarIntWriter;
use parking_lot::Mutex;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::spawn;
use uuid::uuid;

async fn handle_connection(
    socket: &mut TcpStream,
    map: Arc<Mutex<ServerMap>>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    loop {
        let mut buf = [0u8; 8];
        let _ = socket.read_exact(&mut buf).await?;
        let mut push_bst: BTreeSet<PlayerArcWrapper> = BTreeSet::new();
        let name = std::str::from_utf8(&buf[6..8])?;
        push_bst.insert(PlayerArcWrapper::new(Player {
            name: name.to_string(),
            uuid: uuid!("F9168C5E-CEB2-4faa-B6BF-329BF39FA1E4"),
            servers: BTreeSet::new(),
        }));

        let mut lock = map.lock();

        lock.insert(ServerArcWrapper::new(Server {
            addr: SocketAddr::V4(SocketAddrV4::new(
                Ipv4Addr::new(buf[0], buf[1], buf[2], buf[3]),
                u8s_to_u16(buf[4], buf[5]),
            )),
            players: push_bst,
        }))?;

        let found = lock.find(SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::new(buf[0], buf[1], buf[2], buf[3]),
            u8s_to_u16(buf[4], buf[5]),
        )))?;

        // println!("found: {:?}", found.lock());

        match found {
            Some(found) => {
                println!(
                    "found players: {:?}",
                    found
                        .lock()
                        .players
                        .iter()
                        .map(|item| { item.lock().name.clone() })
                        .collect::<Vec<String>>()
                );
            }
            None => {
                println!("Not found");
            }
        }

        drop(lock);
    }
}

async fn serialize_all(map: Arc<Mutex<ServerMap>>) -> Result<(), Box<dyn Error + Send + Sync>> {
    let lock = map.lock();
    let player_array = &lock.player_array;
    let server_array = &lock.server_array;

    tokio::fs::create_dir_all("./data_bin/servers/").await?;

    if tokio::fs::try_exists("./data_bin/players.bin.old").await? {
        tokio::fs::remove_file("./data_bin/players.bin.old").await?;
    }
    if tokio::fs::try_exists("./data_bin/players.bin").await? {
        tokio::fs::rename("./data_bin/players.bin", "./data_bin/players.bin.old").await?;
    }

    let mut player_buf: Vec<u8> = vec![];
    player_buf.write_varint(player_array.len())?;
    for player in player_array {
        player_buf.write_all(&player.serialize()?).await?;
    }

    println!("player_buf: {player_buf:X?}");

    tokio::fs::write("./data_bin/players.bin", player_buf).await?;
    tokio::fs::remove_file("./data_bin/players.bin.old").await?;

    for (ip_a, server_range) in server_array {
        let segment_a: u8;
        let segment_b: u8;
        {
            let segments = (*ip_a).to_be_bytes();
            segment_a = segments[0];
            segment_b = segments[1];
        }
        tokio::fs::create_dir_all(format!("./data_bin/servers/{}/", segment_a)).await?;
        for (_ip_b, ip_servers) in server_range.lock().iter() {
            let mut stack = std::collections::LinkedList::new();
            for (_port, server) in ip_servers {
                let val = server.lock().serialize()?;
                stack.push_back(val);
            }
            let total_len = stack.len();
            let mut total_len_buf = vec![];
            total_len_buf.write_varint(total_len)?;
            let mut file = tokio::fs::File::open(format!(
                "./data_bin/servers/{}/{}.bin",
                segment_a, segment_b
            ))
            .await?;
            file.write_all(&total_len_buf).await?;
            for server in stack {
                file.write_all(&server).await?;
            }
        }
    }

    Ok(())
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

    serialize_all(map.clone()).await.unwrap();

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

#[allow(unused)]
fn u16s_to_u32(a: u16, b: u16) -> u32 {
    ((a as u32) << 16) | b as u32
}

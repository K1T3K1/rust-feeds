use lazy_static::lazy_static;
use textnonce::TextNonce;

use crate::authstore::auth_user;
use crate::messaging::{read_arbitrary_message, read_auth_message, write_info_message};
use std::collections::HashMap;
use std::{net::Shutdown, sync::Arc};

use smol::{
    io::AsyncReadExt,
    net::{TcpListener, TcpStream},
    stream::StreamExt,
    Task,
};
use smol::{
    lock::{Mutex, RwLock},
    Executor,
};
pub static BROKER_NAME: &str = "rust-feeds";
pub static NAME_LENGTH: u8 = BROKER_NAME.len() as u8;

lazy_static! {
    pub static ref SUBS: RwLock<HashMap<String, Vec<Arc<Mutex<TcpStream>>>>> =
        RwLock::new(HashMap::new());
}

pub struct Server {
    listener: TcpListener,
    listener_tasks: Vec<Task<Result<(), std::io::Error>>>,
}

impl Server {
    pub async fn new(port: u32) -> Result<Server, std::io::Error> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
        println!("Bound on port: {:?}", port);

        Ok(Server {
            listener,
            listener_tasks: vec![],
        })
    }

    pub async fn listen(self, executor: Arc<Executor<'static>>) -> Result<(), std::io::Error> {
        let arc_self = Arc::new(Mutex::new(self));
        let exec_arc = Arc::clone(&executor);
        executor.run(loop_listen(arc_self, exec_arc)).await
    }
}

async fn loop_listen(
    server: Arc<Mutex<Server>>,
    executor: Arc<Executor<'static>>,
) -> Result<(), std::io::Error> {
    let server_mtx = server.lock().await;
    println!("Listening on: {:?}", server_mtx.listener.local_addr()?);

    let listener = server_mtx.listener.clone();
    drop(server_mtx);

    let mut incoming = listener.incoming();

    while let Some(stream) = incoming.next().await {
        let stream = stream?;
        println!("Got connection from: {}", stream.peer_addr().unwrap());

        let arc_serv = Arc::clone(&server);
        let conn_arc_exec = Arc::clone(&executor);

        executor
            .spawn(handle_first_connection(arc_serv, stream, conn_arc_exec))
            .detach();
    }
    Ok(())
}

async fn handle_first_connection(
    server: Arc<Mutex<Server>>,
    mut stream: TcpStream,
    executor: Arc<Executor<'static>>,
) {
    let nonce: TextNonce;
    match write_info_message(&mut stream).await {
        Ok(n) => nonce = n,
        Err(_) => {
            let _ = stream.shutdown(Shutdown::Both);
            return;
        }
    }

    let auth_data: Vec<u8>;
    match read_auth_message(&mut stream).await {
        Ok(adata) => auth_data = adata,
        Err(_) => {
            let _ = stream.shutdown(Shutdown::Both);
            return;
        }
    }

    let owner_shift = 6 + auth_data[5] as usize;
    let owner_name = &auth_data[6..owner_shift];
    let owner_name_str: &str;
    match std::str::from_utf8(owner_name) {
        Ok(on) => owner_name_str = on,
        Err(_) => return,
    }

    let user_sha = &auth_data[owner_shift..];
    if auth_user(owner_name_str, nonce.as_bytes(), user_sha) {
        let mut server_mtx = server.lock().await;
        let listen_future = executor.spawn(listen_to_client(stream));
        server_mtx.listener_tasks.push(listen_future);
        println!("User {} authenticated!", owner_name_str);
    } else {
        println!("Failed to authenticate user {}", owner_name_str);
    }
}

async fn listen_to_client(stream: TcpStream) -> Result<(), std::io::Error> {
    let mut buff = [0u8; 4];
    let mut reader_half = stream.clone();
    let writer_half = Arc::new(Mutex::new(stream.clone()));

    loop {
        reader_half.read_exact(&mut buff).await?;
        read_arbitrary_message(&mut reader_half, &writer_half, u32::from_be_bytes(buff)).await?;
    }
}

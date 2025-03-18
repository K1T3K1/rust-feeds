use textnonce::TextNonce;

use crate::authstore::auth_user;
use std::{net::Shutdown, sync::Arc};

use smol::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    stream::StreamExt,
    Task,
};
use smol::{lock::Mutex, Executor};

static BROKER_NAME: &str = "rust-feeds";
static NAME_LENGTH: u8 = BROKER_NAME.len() as u8;

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

async fn listen_to_client(mut stream: TcpStream) -> Result<(), std::io::Error> {
    todo!();
    let mut buff = [0u8; 8];
    stream.read_exact(&mut buff).await?;

    println!("{:?}", buff);
    Ok(())
}

#[inline(always)]
async fn write_info_message(stream: &mut TcpStream) -> Result<TextNonce, std::io::Error> {
    let nonce = TextNonce::new();
    let total_len = 6 + 32 + NAME_LENGTH as usize;
    let total_len_32 = total_len as u32;
    let mut data: Vec<u8> = Vec::with_capacity(total_len);
    data.extend_from_slice(&total_len_32.to_be_bytes());
    data.push(1);
    data.push(NAME_LENGTH);
    data.extend_from_slice(&BROKER_NAME.as_bytes());
    data.extend_from_slice(&nonce.as_bytes());

    stream.write_all(&data).await?;

    return Ok(nonce);
}

#[inline(always)]
async fn read_auth_message(stream: &mut TcpStream) -> Result<Vec<u8>, std::io::Error> {
    let mut auth_buf = [0u8; 4];
    stream.read_exact(&mut auth_buf).await?;
    let len = u32::from_be_bytes(auth_buf);

    if len < 39 {
        write_error_message(
            stream,
            &format!("Expected at least 34 bytes. Got: {}.", auth_buf.len()),
        )
        .await?;
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Data length less than excepted",
        ));
    }

    let mut data_buf = vec![0u8; (len - 4) as usize];
    stream.read_exact(&mut data_buf).await?;

    if data_buf[0] != 2 {
        write_error_message(
            stream,
            &format!("Invalid Error Code. Expected 2. Got: {}.", auth_buf[4]),
        )
        .await?;
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Wrong op code provided",
        ));
    }

    let mut data = Vec::with_capacity(len as usize);
    data.extend_from_slice(&auth_buf);
    data.extend_from_slice(&data_buf);

    return Ok(data);
}

async fn write_error_message(
    stream: &mut TcpStream,
    error_message: &str,
) -> Result<(), std::io::Error> {
    let capacity = 5 + error_message.len();
    let mut data: Vec<u8> = Vec::with_capacity(capacity);
    data.extend_from_slice(&capacity.to_be_bytes());
    data.push(0);
    data.extend_from_slice(&error_message.as_bytes());

    stream.write_all(&data).await?;
    Ok(())
}

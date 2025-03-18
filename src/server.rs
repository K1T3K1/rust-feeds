use textnonce::TextNonce;

use crate::authstore::auth_user;
use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{Shutdown, SocketAddr},
    sync::Arc, time::Duration
};

use smol::{io::{AsyncReadExt, AsyncWriteExt}, net::{TcpListener, TcpStream}, stream::StreamExt, Timer};
use smol::Executor;

static BROKER_NAME: &str = "rust-feeds";
static NAME_LENGTH: u8 = BROKER_NAME.len() as u8;

pub struct Server {
    executor: Arc<Executor<'static>>,
    listener: TcpListener,
    subs: HashMap<String, Vec<TcpStream>>,
}

impl Server {
    pub async fn new(port: u32, executor: Arc<Executor<'static>>) -> Result<Server, std::io::Error> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
        println!("Bound on port: {:?}", port);

        Ok(Server {
            executor,
            listener,
            subs: HashMap::new(),
        })
    }

    pub async fn listen(&self) -> Result<(), std::io::Error> {
        self.executor.run(self.loop_listen()).await
    }

    async fn loop_listen(&self) -> Result<(), std::io::Error> {
        println!("Listening on: {:?}", self.listener.local_addr()?);
        let mut incoming = self.listener.incoming();
        while let Some(stream) = incoming.next().await {
            let stream = stream?;
            println!("Got connection from: {}", stream.peer_addr().unwrap());

            self.executor.spawn(async move {
                let _ = handle_first_connection(stream).await;
            }).detach();
        }
        Ok(())
    }
}

async fn handle_first_connection(mut stream: TcpStream) {
    let nonce: TextNonce;
    println!("Handling conn...");
    match write_info_message(&mut stream).await {
        Ok(n) => nonce = n,
        Err(_) => return,
    }

    println!("Wrote info message...");

    let auth_data: Vec<u8>;
    match read_auth_message(&mut stream).await {
        Ok(adata) => auth_data = adata,
        Err(_) => {
            let _ = stream.shutdown(Shutdown::Both);
            return;
        },
    }

    println!("Read auth message...");

    let owner_shift = 6 + auth_data[5] as usize;
    let owner_name = &auth_data[6..owner_shift];
    let owner_name_str: &str;
    match std::str::from_utf8(owner_name) {
        Ok(on) => owner_name_str = on,
        Err(_) => return,
    }

    let user_sha = &auth_data[owner_shift..];
    if auth_user(owner_name_str, nonce.as_bytes(), user_sha) {
        println!("User {} authenticated!", owner_name_str);
    } else {
        println!("Failed to authenticate user {}", owner_name_str);
    }
}

#[inline(always)]
async fn write_info_message(stream: &mut TcpStream) -> Result<TextNonce, &str> {
    let nonce = TextNonce::new();
    let total_len = 6 + 32 + NAME_LENGTH as usize;
    let total_len_32 = total_len as u32;
    let mut data: Vec<u8> = Vec::with_capacity(total_len);
    data.extend_from_slice(&total_len_32.to_be_bytes());
    data.push(1);
    data.push(NAME_LENGTH);
    data.extend_from_slice(&BROKER_NAME.as_bytes());
    data.extend_from_slice(&nonce.as_bytes());

    if let Err(_) = stream.write_all(&data).await {
        println!(
            "Failed to write info data to host: {}",
            stream.peer_addr().unwrap_or(SocketAddr::new(
                std::net::IpAddr::V4(std::net::Ipv4Addr::new(1, 1, 1, 1)),
                1
            ))
        );
        stream.shutdown(Shutdown::Both);
        let _ = stream.shutdown(Shutdown::Both);
        return Err("Failed to read auth data");
    }
    return Ok(nonce);
}

#[inline(always)]
async fn read_auth_message(stream: &mut TcpStream) -> Result<Vec<u8>, &'static str> {
    let mut auth_buf = Vec::with_capacity(60);
    if let Err(_) = stream.read_to_end(&mut auth_buf).await {
        println!(
            "Failed to read auth data from host: {}",
            stream.peer_addr().unwrap_or(SocketAddr::new(
                std::net::IpAddr::V4(std::net::Ipv4Addr::new(1, 1, 1, 1)),
                1
            ))
        );
        stream.shutdown(Shutdown::Both);
        return Err("Failed to read auth data");
    }

    if auth_buf.len() < 39 {
        write_error_message(stream, &format!("Expected at least 39 bytes. Got: {}.", auth_buf.len()));
        return Err("Not enough data");
    }

    if auth_buf[4] != 2 {
        println!(
            "Got OpCode: {} | From: {} | When Auth (2) was expected",
            auth_buf[4],
            stream.peer_addr().unwrap_or(SocketAddr::new(
                std::net::IpAddr::V4(std::net::Ipv4Addr::new(1, 1, 1, 1)),
                1
            ))
        );
        stream.shutdown(Shutdown::Both);
        write_error_message(stream, &format!("Invalid Error Code. Expected 2. Got: {}.", auth_buf[4]));
        return Err("Invalid Op Code");
    }

    return Ok(auth_buf);
}

async fn write_error_message(stream: &mut TcpStream, error_message: &str) {
    let capacity = 5 + error_message.len();
    let mut data: Vec<u8> = Vec::with_capacity(capacity);
    data.extend_from_slice(&capacity.to_be_bytes());
    data.push(0);
    data.extend_from_slice(&error_message.as_bytes());

    if let Err(_) = stream.write_all(&data).await  {
        println!(
            "Failed to write error data to host: {}",
            stream.peer_addr().unwrap_or(SocketAddr::new(
                std::net::IpAddr::V4(std::net::Ipv4Addr::new(1, 1, 1, 1)),
                1
            ))
        );
    }
}


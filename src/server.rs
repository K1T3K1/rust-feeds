use textnonce::TextNonce;

use crate::{authstore::auth_user, threadpool::ThreadPool};
use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{Shutdown, SocketAddr, TcpListener, TcpStream},
};

static BROKER_NAME: &str = "rust-feeds";
static NAME_LENGTH: u8 = BROKER_NAME.len() as u8;

pub struct Server {
    pool: ThreadPool,
    listener: TcpListener,
    subs: HashMap<String, Vec<TcpStream>>,
}

impl Server {
    pub fn new(port: u32, pool: ThreadPool) -> Server {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).unwrap();
        println!("Bound on port: {:?}", port);

        Server {
            pool,
            listener,
            subs: HashMap::new(),
        }
    }

    pub fn listen(&self) {
        println!("Listening on: {:?}", self.listener.local_addr().unwrap());
        for stream in self.listener.incoming() {
            let stream = stream.unwrap();
            println!("Got connection from: {}", stream.peer_addr().unwrap());

            self.pool.execute(|| handle_first_connection(stream));
        }
    }
}

fn handle_first_connection(mut stream: TcpStream) {
    let nonce: TextNonce;
    match write_info_message(&mut stream) {
        Ok(n) => nonce = n,
        Err(_) => return,
    }

    let auth_data: Vec<u8>;
    match read_auth_message(&mut stream) {
        Ok(adata) => auth_data = adata,
        Err(_) => {
            let _ = stream.shutdown(Shutdown::Both);
            return;
        },
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
        println!("User {} authenticated!", owner_name_str);
    } else {
        println!("Failed to authenticate user {}", owner_name_str);
    }
}

#[inline(always)]
fn write_info_message(stream: &mut TcpStream) -> Result<TextNonce, &str> {
    let nonce = TextNonce::new();
    let total_len = 6 + 32 + NAME_LENGTH as usize;
    let total_len_32 = total_len as u32;
    let mut data: Vec<u8> = Vec::with_capacity(total_len);
    data.extend_from_slice(&total_len_32.to_be_bytes());
    data.push(1);
    data.push(NAME_LENGTH);
    data.extend_from_slice(&BROKER_NAME.as_bytes());
    data.extend_from_slice(&nonce.as_bytes());

    if let Err(_) = stream.write_all(&data) {
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
fn read_auth_message(stream: &mut TcpStream) -> Result<Vec<u8>, &'static str> {
    let mut auth_buf = Vec::with_capacity(60);
    if let Err(_) = stream.read_to_end(&mut auth_buf) {
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

fn write_error_message(stream: &mut TcpStream, error_message: &str) {
    let capacity = 5 + error_message.len();
    let mut data: Vec<u8> = Vec::with_capacity(capacity);
    data.extend_from_slice(&capacity.to_be_bytes());
    data.push(0);
    data.extend_from_slice(&error_message.as_bytes());

    if let Err(_) = stream.write_all(&data) {
        println!(
            "Failed to write error data to host: {}",
            stream.peer_addr().unwrap_or(SocketAddr::new(
                std::net::IpAddr::V4(std::net::Ipv4Addr::new(1, 1, 1, 1)),
                1
            ))
        );
    }
}


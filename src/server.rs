use crate::threadpool::ThreadPool;
use std::{io::{BufReader, Read}, net::{Shutdown, TcpListener, TcpStream}};

pub struct Server {
    pool: ThreadPool,
    listener: TcpListener
}

impl Server {
    pub fn new(port: u32, pool: ThreadPool) -> Server {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).unwrap();
        println!("Bound on port: {:?}", port);
        Server { pool, listener }
    }

    pub fn listen(&self) {
        println!("Listening on: {:?}", self.listener.local_addr().unwrap());
        for stream in self.listener.incoming() {
            let stream = stream.unwrap();

            self.pool.execute(||
                handle_connection(stream)
            );
        }
    }
}

fn handle_connection(stream: TcpStream) {
    let mut buf_reader = BufReader::new(&stream);
    let mut data_size: [u8;4] = [0;4];
    buf_reader.read_exact(&mut data_size).unwrap();

    let parsed_size = u32::from_be_bytes(data_size);

    let mut data = vec![0;parsed_size as usize];
    buf_reader.read_exact(&mut data).unwrap();

    println!("{:?}", data);
    stream.shutdown(Shutdown::Both).unwrap();
}

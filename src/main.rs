use server::Server;
use threadpool::ThreadPool;

mod threadpool;
mod server;
mod authstore;
mod sqlite_authstore;

fn main() {
    println!("Starting app...");
    println!("Creating threadpool...");
    let pool = ThreadPool::new(6);
    let server = Server::new(2137, pool);
    server.listen();
}

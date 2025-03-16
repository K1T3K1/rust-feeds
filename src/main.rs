use authstore::AuthStoreSource;
use server::Server;
use sqlite_authstore::SqliteAuthStore;
use threadpool::ThreadPool;

mod authstore;
mod server;
mod sqlite_authstore;
mod threadpool;

fn main() {
    let auth_store = SqliteAuthStore {};
    auth_store.feed_cache();
    println!("Starting app...");
    println!("Creating threadpool...");
    let pool = ThreadPool::new(6);
    let server = Server::new(2137, pool);
    server.listen();
}

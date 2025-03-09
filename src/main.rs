use server::Server;
use sqlite_authstore::SqliteAuthStore;
use threadpool::ThreadPool;
use authstore::AuthStoreSource;

mod threadpool;
mod server;
mod authstore;
mod sqlite_authstore;

fn main() {
    let auth_store = SqliteAuthStore{};
    auth_store.feed_cache();
    println!("Starting app...");
    println!("Creating threadpool...");
    let pool = ThreadPool::new(6);
    let server = Server::new(2137, pool);
    server.listen();
}

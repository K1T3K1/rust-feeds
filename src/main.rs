use authstore::AuthStoreSource;
use server::Server;
use sqlite_authstore::SqliteAuthStore;
use smol::Executor;
use smol::block_on;
use std::sync::Arc;

mod authstore;
mod server;
mod sqlite_authstore;
mod threadpool;

fn main() {
    let auth_store = SqliteAuthStore {};
    auth_store.feed_cache();
    println!("Starting app...");
    let executor = Arc::new(Executor::new());
    if let Ok(server) = block_on(Server::new(2137, executor)) {
        block_on(server.listen());
    } else {
        println!("Failed to start server. Shutting down...");
    }
    
}

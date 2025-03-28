use authstore::AuthStoreSource;
use server::Server;
use smol::Executor;
use smol_macros::main;
use sqlite_authstore::SqliteAuthStore;
use std::sync::Arc;
mod authstore;
mod message_string;
mod messaging;
mod server;
mod sqlite_authstore;

main! { async fn main() {
    let auth_store = SqliteAuthStore {};
    auth_store.feed_cache();
    println!("Starting app...");
    let executor = Arc::new(Executor::new());
    if let Ok(server) = Server::new(2137).await {
        server.listen(executor).await;
    } else {
        println!("Failed to start server. Shutting down...");
    }

}
}

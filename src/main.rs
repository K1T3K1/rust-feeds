use authstore::AuthStoreSource;
use server::Server;
use smol::Executor;
use sqlite_authstore::SqliteAuthStore;
use std::sync::Arc;
use smol_macros::main;
mod authstore;
mod server;
mod sqlite_authstore;
mod threadpool;

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

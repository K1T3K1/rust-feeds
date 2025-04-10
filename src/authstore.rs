use lazy_static::lazy_static;
use sha2::{Digest, Sha256};
use smol::lock::RwLock;
use std::collections::{HashMap, HashSet};

pub struct AuthDbObject {
    pub owner: String,
    pub secret: String,
    pub allow_sub: String,
    pub allow_pub: String,
}

pub struct AuthObject {
    pub secret: String,
    pub allow_sub: HashSet<String>,
    pub allow_pub: HashSet<String>,
}
lazy_static! {
    pub static ref AUTH_MAP: RwLock<HashMap<String, AuthObject>> =
        RwLock::new(HashMap::with_capacity(32));
}
pub trait AuthStoreSource {
    async fn feed_cache(&self);
    async fn update_cache(&self);
}

#[inline(always)]
fn generate_sha(secret: &str, nonce: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();

    let mut hash = [0; 32];
    hasher.update(nonce);
    hasher.update(secret.as_bytes());
    hash.copy_from_slice(hasher.finalize().as_slice());
    hash
}

#[inline(always)]
pub async fn auth_user(owner: &str, nonce: &[u8], user_sha: &[u8]) -> bool {
    let map = AUTH_MAP.read().await;
    match map.get(owner) {
        None => false,
        Some(auth_object) => generate_sha(&auth_object.secret, nonce) == *user_sha,
    }
}

#[inline(always)]
pub async fn auth_pub(owner: &str, pub_chan: &str) -> bool {
    let map = AUTH_MAP.read().await;
    match map.get(owner) {
        None => false,
        Some(auth_object) => auth_object.allow_pub.contains(pub_chan),
    }
}

#[inline(always)]
pub async fn auth_sub(owner: &str, sub_chan: &str) -> bool {
    let map = AUTH_MAP.read().await;
    match map.get(owner) {
        None => false,
        Some(auth_object) => auth_object.allow_sub.contains(sub_chan),
    }
}

use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, RwLock};
use sha2::{Sha256, Digest};
use textnonce::TextNonce;

pub struct AuthDbObject {
    pub owner: String,
    pub secret: String,
    pub allow_sub: String,
    pub allow_pub: String
}

pub struct AuthObject {
    pub sha: [u8; 32],
    pub allow_sub: HashSet<String>,
    pub allow_pub: HashSet<String>
}

pub static AUTH_MAP: LazyLock<RwLock<HashMap<String, AuthObject>>> = LazyLock::new(|| RwLock::new(HashMap::with_capacity(32)));
static NONCE: LazyLock<TextNonce> = LazyLock::new(|| textnonce::TextNonce::new());

pub trait AuthStoreSource {
    fn feed_cache(&self);
    fn update_cache(&self);
}

#[inline(always)]
pub fn generate_sha(secret: &str) -> [u8;32] {
    let mut hasher = Sha256::new();
    let mut byte_string = Vec::with_capacity(32 + secret.len());
    byte_string.extend_from_slice(NONCE.as_bytes());
    byte_string.extend_from_slice(secret.as_bytes());
    hasher.update(byte_string.as_slice());
    
    let mut hash = [0;32];
    hash.copy_from_slice(hasher.finalize().as_slice());
    hash
}

#[inline(always)]
pub fn auth_user(owner: &str, secret: &[u8;32]) -> bool {
    let map = AUTH_MAP.read().unwrap();
    match map.get_key_value(owner) {
        None => false,
        Some((_, auth_object)) => auth_object.sha == *secret 
    }
}

#[inline(always)]
pub fn auth_pub(owner: &str, pub_chan: &str) -> bool {
    let map = AUTH_MAP.read().unwrap();
    match map.get_key_value(owner) {
        None => false,
        Some((_, auth_object)) => auth_object.allow_pub.contains(pub_chan)
    }
}

#[inline(always)]
pub fn auth_sub(owner: &str, sub_chan: &str) -> bool {
    let map = AUTH_MAP.read().unwrap();
    match map.get_key_value(owner) {
        None => false,
        Some((_, auth_object)) => auth_object.allow_sub.contains(sub_chan)
    }
}


use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, RwLock};

use textnonce::TextNonce;

struct AuthObject {
    sha: [u32; 5],
    allow_sub: HashSet<String>,
    allow_pub: HashSet<String>
}

static AUTH_MAP: LazyLock<RwLock<HashMap<String, AuthObject>>> = LazyLock::new(|| RwLock::new(HashMap::with_capacity(32)));
static NONCE: LazyLock<TextNonce> = LazyLock::new(|| textnonce::TextNonce::new());

trait AuthStoreSource {
    fn feed_cache();
}
#[inline(always)]
fn auth_user(owner: &str, secret: &[u32;5]) -> bool {
    let map = AUTH_MAP.read().unwrap();
    match map.get_key_value(owner) {
        None => false,
        Some((_, auth_object)) => auth_object.sha == *secret 
    }
}

#[inline(always)]
fn auth_pub(owner: &str, pub_chan: &str) -> bool {
    let map = AUTH_MAP.read().unwrap();
    match map.get_key_value(owner) {
        None => false,
        Some((_, auth_object)) => auth_object.allow_pub.contains(pub_chan)
    }
}

#[inline(always)]
fn auth_sub(owner: &str, sub_chan: &str) -> bool {
    let map = AUTH_MAP.read().unwrap();
    match map.get_key_value(owner) {
        None => false,
        Some((_, auth_object)) => auth_object.allow_sub.contains(sub_chan)
    }
}

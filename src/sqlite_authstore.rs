use crate::authstore::{AuthDbObject, AuthObject, AuthStoreSource, AUTH_MAP};
use rusqlite::Connection;

pub struct SqliteAuthStore {}

static QUERY: &str = "SELECT owner, secret, allow_sub, allow_pub 
             FROM auth_objects;";

impl AuthStoreSource for SqliteAuthStore {
    fn feed_cache(&self) {
        let conn = Connection::open("/app/sqlite/auth.db").unwrap();

        let mut stmt = conn.prepare(QUERY).unwrap();

        let owners = stmt
            .query_map([], |row| {
                Ok(AuthDbObject {
                    owner: row.get(0)?,
                    secret: row.get(1)?,
                    allow_sub: row.get(2)?,
                    allow_pub: row.get(3)?,
                })
            })
            .unwrap();

        let mut map = AUTH_MAP.write().unwrap();
        for owner in owners {
            let owner = owner.unwrap();
            map.insert(
                owner.owner,
                AuthObject {
                    secret: owner.secret,
                    allow_sub: owner
                        .allow_sub
                        .split(",")
                        .map(|s| s.trim().to_string())
                        .collect(),
                    allow_pub: owner
                        .allow_pub
                        .split(",")
                        .map(|s| s.trim().to_string())
                        .collect(),
                },
            );
        }
    }

    fn update_cache(&self) {
        let conn = Connection::open("/app/sqlite/auth.db").unwrap();
        let mut stmt = conn.prepare(QUERY).unwrap();

        let owners = stmt
            .query_map([], |row| {
                Ok(AuthDbObject {
                    owner: row.get(0)?,
                    secret: row.get(1)?,
                    allow_sub: row.get(2)?,
                    allow_pub: row.get(3)?,
                })
            })
            .unwrap();

        let map = AUTH_MAP.read().unwrap();

        let diff_owners: Vec<AuthDbObject> = owners
            .filter_map(|owner_result| match owner_result {
                Ok(owner) => {
                    if !map.contains_key(&owner.owner) {
                        Some(owner)
                    } else {
                        None
                    }
                }
                Err(_) => None,
            })
            .collect();

        let mut map = AUTH_MAP.write().unwrap();
        for owner in diff_owners {
            let owner = owner;
            map.insert(
                owner.owner,
                AuthObject {
                    secret: owner.secret,
                    allow_sub: owner
                        .allow_sub
                        .split(",")
                        .map(|s| s.trim().to_string())
                        .collect(),
                    allow_pub: owner
                        .allow_pub
                        .split(",")
                        .map(|s| s.trim().to_string())
                        .collect(),
                },
            );
        }
    }
}

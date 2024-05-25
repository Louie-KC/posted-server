use std::thread;

use std::sync::mpsc;

use log::{info, warn};
use uuid::Uuid;

use crate::cache::cache::{Cache, Entry};
use super::backup_auth::OfflineAuth;
use super::redis_auth::RedisAuth;

const MAX_CONNECT_TIME: u64 = 1;
const RECONNECT_FREQUENCY: u64 = 1;

enum Store {
    Online(RedisAuth),
    Offline(OfflineAuth)
}

pub struct AuthService {
    store: Store,
    addr: String,
    misses: u64
}

impl AuthService {
    pub fn new(addr: &str) -> AuthService {
        let store = match try_connect(addr) {
            Ok(redis_cache) => Store::Online(RedisAuth::new(redis_cache)),
            Err(_) => Store::Offline(OfflineAuth::new()),
        };

        AuthService { store, addr: addr.to_string(), misses: 0 }
    }

    async fn maybe_reconnect(&mut self) -> () {
        if self.misses % RECONNECT_FREQUENCY != 0 {
            return
        }
        info!("AuthService: Offline & re-connect frequency met. Misses: {}", self.misses);
        info!("AuthService: Attempting to (re)connect to '{}'", self.addr);

        if let Store::Offline(offline) = &self.store {
            if let Ok(redis_cache) = try_connect(&self.addr) {
                if let Err(_) = migrate_to_online(offline, &redis_cache).await {
                    warn!("AuthService: attempted but failed to migrate to Redis server");
                    return
                }
                self.store = Store::Online(RedisAuth::new(redis_cache));
                self.misses = 0;
                info!("AuthService: re-connected and migrated to Redis server")
            } else {
                info!("AuthService: failed to re-connect to '{}'", self.addr)
            }
        }
    
    }

    pub async fn generate_user_token(&mut self, user_id: u64, username: &str) -> Result<Uuid, ()> {
        if let Store::Offline(_) = &self.store {
            self.maybe_reconnect().await;
        }

        match &mut self.store {
            Store::Offline(store) => {
                self.misses += 1;
                Ok(store.generate_for_user(user_id))
            },
            Store::Online(redis)  => {
                let result = redis.generate_for_user(user_id, username).await;
                if let Ok(stored_uuid) = result {
                    Ok(stored_uuid)
                } else {
                    let mut offline = OfflineAuth::new();
                    let stored_uuid = offline.generate_for_user(user_id);
                    self.store = Store::Offline(offline);
                    self.misses = 1;
                    Ok(stored_uuid)
                }
            },
        }
    }

    pub async fn validate(&mut self, user_id: u64, username: &str, token_str: &str) -> Result<bool, ()> {
        let token = match Uuid::parse_str(token_str) {
            Ok(uuid) => uuid,
            Err(_) => return Err(()),
        };

        if let Store::Offline(_) = &self.store {
            self.maybe_reconnect().await;
        }

        match &self.store {
            Store::Offline(store) => {
                self.misses += 1;
                Ok(store.validate(user_id, token))
            },
            Store::Online(redis)  => {
                let result = redis.validate_username(username, token).await;
                if let Ok(is_valid) = result {
                    return Ok(is_valid)
                } else {
                    warn!("AuthService: Switching to OfflineAuth");
                    self.store = Store::Offline(OfflineAuth::new());
                    self.misses = 1;
                    Err(())
                }
            },
        }
    }

}

fn try_connect(addr: &str) -> Result<Cache, ()> {
    let (sender, receiver) = mpsc::channel();
    
    let _ = thread::scope(|s: &thread::Scope<'_, '_>| {
        s.spawn(|| {
            let _ = sender.send(Cache::new(addr));
        });
    });

    match receiver.recv_timeout(std::time::Duration::from_secs(MAX_CONNECT_TIME)) {
        Ok(conn_result) => conn_result,
        Err(_) => {
            warn!("AuthService::try_connect({}): connection failed", addr);
            Err(())
        },
    }
}

async fn migrate_to_online(offline: &OfflineAuth, online: &Cache) -> Result<(), ()> {
    let entries = offline.tokens.iter()
                                .map(|entry| Entry {
                                    key: entry.0.to_string(),
                                    value: entry.1.to_string(),
                                    expiry_sec: 120
                                })
                                .collect();
    match online.set_multiple(entries, false, true).await {
        Ok(_)  => Ok(()),
        Err(_) => Err(()),
    }
}
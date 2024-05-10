use std::thread;

use std::sync::mpsc;

use log::{info, warn};
use uuid::Uuid;

use crate::cache::cache::Cache;
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
        if self.misses % RECONNECT_FREQUENCY == 0 {
            info!("AuthService: Offline & re-connect frequency met. Misses: {}", self.misses);
            info!("AuthService: Attempting to (re)connect to '{}'", self.addr);
            if let Ok(redis_cache) = try_connect(&self.addr) {
                self.store = Store::Online(RedisAuth::new(redis_cache));
                self.misses = 0;
                info!("AuthService: connection to Redis server established")
            } else {
                info!("AuthService: failed to (re)connect to '{}'", self.addr)
            }
        }
    }

    pub async fn generate_user_token(&mut self, user_id: u64) -> Result<Uuid, ()> {
        if let Store::Offline(_) = &self.store {
            self.maybe_reconnect().await;
        }

        match &mut self.store {
            Store::Offline(store) => {
                self.misses += 1;
                Ok(store.generate_for_user(user_id))
            },
            Store::Online(redis)  => {
                let result = redis.generate_for_user(user_id).await;
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

    pub async fn validate(&mut self, user_id: u64, token_str: &str) -> Result<bool, ()> {
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
                let result = redis.validate(user_id, token).await;
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
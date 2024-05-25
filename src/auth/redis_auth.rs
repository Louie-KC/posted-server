use uuid::Uuid;

use crate::cache::cache::{Cache, Entry};

const DAY_IN_SECONDS: u64 = 60 * 60 * 12;

pub struct RedisAuth {
    redis_cache: Cache
}

impl RedisAuth {
    pub fn new(redis_cache: Cache) -> Self {
        RedisAuth { redis_cache: redis_cache }
    }

    pub async fn generate_for_user(&self, user_id: u64, username: &str) -> Result<Uuid, ()> {
        let uuid = Uuid::new_v4();
        let token_to_user = create_token_to_user_entry(&uuid, username, user_id);
        let user_to_token = create_user_to_token_entry(username, &uuid, user_id);
        match self.redis_cache.set_multiple(vec![token_to_user, user_to_token], false, true).await {
            Ok(_)  => Ok(uuid),
            Err(_) => Err(()),
        }
        // let entry = Entry {
        //     uuid: uuid,
        //     user_id: user_id,
        //     expiry_sec: DAY_IN_SECONDS
        // };
        // match self.redis_cache.set_single(entry, false, true).await {
        //     Ok(_) => Ok(uuid),
        //     Err(_) => Err(()),
        // }
    }

    /// Determines whether a `user_id` has a token mapped to it, and if it so, compares
    /// `token` to it. `true` is returned if the mapped token matches the `token` parameter.
    /// `false` is returned if there is no mapping, or the provided `token` does not match.
    pub async fn validate(&self, user_id: u64, token: Uuid) -> Result<bool, ()> {
        let Ok(user_token) = self.redis_cache.get_token_by_user_id(user_id).await else {
            return Err(())
        };
        // info!("token retrieved from Redis server");
        Ok(Uuid::eq(&user_token, &token))
    }
}

fn create_token_to_user_entry(token: &Uuid, username: &str, user_id: u64) -> Entry {
    Entry::new(token.to_string(), format!("{}!{}", username, user_id), DAY_IN_SECONDS)
}

fn create_user_to_token_entry(username: &str, token: &Uuid, user_id: u64) -> Entry {
    Entry::new(username.to_string(), format!("{}!{}", token.to_string(), user_id), DAY_IN_SECONDS)
}
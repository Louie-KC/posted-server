use uuid::Uuid;

use crate::cache::cache::{Cache, UserToken};

const DAY_IN_SECONDS: u64 = 60 * 60 * 12;

pub struct RedisAuth {
    redis_cache: Cache
}

impl RedisAuth {
    pub fn new(redis_cache: Cache) -> Self {
        RedisAuth { redis_cache: redis_cache }
    }

    pub async fn generate_for_user(&self, user_id: u64) -> Result<Uuid, ()> {
        let uuid = Uuid::new_v4();
        let entry = UserToken {
            uuid: uuid,
            user_id: user_id,
            expiry_sec: DAY_IN_SECONDS
        };
        match self.redis_cache.set_single(entry, false, true).await {
            Ok(_) => Ok(uuid),
            Err(_) => Err(()),
        }
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
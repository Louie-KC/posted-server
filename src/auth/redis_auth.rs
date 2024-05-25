use uuid::Uuid;

use crate::cache::{cache::{Cache, Entry}, error::CacheErr};

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
    }

    pub async fn validate_username(&self, username: &str, token: Uuid) -> Result<bool, ()> {
        let value = match self.redis_cache.get(&token.to_string()).await {
            Ok(value) => value,
            Err(CacheErr::NilResponse) => return Ok(false),
            Err(_) => return Err(())
        };

        let (stored_username, _) = separate_token_result(value)?;

        Ok(stored_username.eq(username))
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

/// `value` in the format of: `<username>!<user_id>`
/// 
/// If successful, returns: (Username, user_id)
fn separate_token_result(value: String) -> Result<(String, u64), ()> {
    let (left, right) = match value.split_once("!") {
        Some((l, r)) => (l, r),
        None => return Err(())
    };

    if left.is_empty() || right.is_empty() || right.contains("!") {
        return Err(())
    }

    match right.parse::<u64>() {
        Ok(id) => Ok((left.to_string(), id)),
        Err(_) => Err(())
    }
}

/// `value` in the format of: `<token>!<user_id>`
fn _separate_user_result(value: String) -> Result<(Uuid, u64), ()> {
    let (left, right) = separate_token_result(value)?;
    match Uuid::parse_str(&left) {
        Ok(uuid) => Ok((uuid, right)),
        Err(_) => Err(())
    }
}
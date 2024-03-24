use uuid::Uuid;

use crate::cache::cache::Cache;

const DAY_IN_SECONDS: u64 = 60 * 60 * 12;

pub async fn generate_for_user(cache: &Cache, user_id: u64) -> Result<Uuid, ()> {
    let uuid = Uuid::new_v4();
    match cache.set_key(&user_id.to_string(), &uuid.as_u128().to_string(), DAY_IN_SECONDS).await {
        Ok(_)  => Ok(uuid),
        Err(_) => Err(())
    }
}

pub async fn validate(cache: &Cache, user_id: u64, token_to_check: Uuid) -> bool {
    let Ok(user_token) = cache.get_token_by_user_id(user_id).await else {
        return false
    };
    Uuid::eq(&user_token, &token_to_check)
}

/// Validates a token string. If the token_string cannot be converted to a
/// UUID (e.g. bad format), then an Err is returned.
pub async fn validate_str(cache: &Cache, user_id: u64, token_string: &str) -> Result<bool, ()> {
    match Uuid::parse_str(token_string) {
        Ok(uuid) => Ok(validate(cache, user_id, uuid).await),
        Err(_) => Err(())
    }
}
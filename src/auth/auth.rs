use std::collections::HashMap;

use uuid::Uuid;

pub type TokenRegistry = HashMap<u64, Uuid>;

pub struct AuthService {
    tokens: TokenRegistry
}

impl AuthService {
    pub fn new() -> Self {
        AuthService { tokens: HashMap::new() }
    }

    /// Generates a new v4 uuid and inserts into the token registry with the
    /// provided `user_id` as the key, and the generated uuid as the value.
    /// 
    /// The generated and registered uuid is returned.
    pub fn generate_for_user(&mut self, user_id: u64) -> Uuid {
        let uuid = Uuid::new_v4();
        self.tokens.insert(user_id, uuid);
        uuid
    }

    /// Verifies whether a provided `token_string` is a valid token for a `user_id`.
    /// 
    /// `false` is returned when the `user_id` has no associated token, or the
    /// associated token does not match the provided `token_to_check`.
    pub fn validate(&self, user_id: u64, token_to_check: Uuid) -> bool {
        match self.tokens.get(&user_id) {
            Some(registered) => registered.eq(&token_to_check),
            None => false
        }
    }

    /// Verifies whether a provided `token_string` is a valid token for a `user_id`.
    /// 
    /// A proxy for AuthService::validate(u64, Uuid) which parses the `token_string`
    /// to a Uuid.
    pub fn validate_str(&self, user_id: u64, token_string: &str) -> Result<bool, ()> {
        match Uuid::parse_str(token_string) {
            Ok(uuid) => Ok(self.validate(user_id, uuid)),
            Err(_) => Err(())
        }
    }
}
use std::collections::HashMap;

use uuid::Uuid;

type TokenRegistry = HashMap<u64, Uuid>;

pub struct OfflineAuth {
    pub(super) tokens: TokenRegistry
}

impl OfflineAuth {
    pub fn new() -> Self {
        OfflineAuth { tokens: HashMap::new() }
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

    /// Verifies whether a provided `token` is a valid token for a `user_id`.
    /// 
    /// `false` is returned when the `user_id` has no associated token, or the
    /// associated token does not match the provided `token_to_check`.
    pub fn validate(&self, user_id: u64, token: Uuid) -> bool {
        match self.tokens.get(&user_id) {
            Some(registered) => registered.eq(&token),
            None => false
        }
    }

}
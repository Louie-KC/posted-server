use log::warn;

use uuid::Uuid;

use redis::{aio::MultiplexedConnection, AsyncCommands};

pub struct UserToken {
    uuid: Uuid,
    user_id: u64,
    expiry_sec: u64
}

pub struct Cache {
    client: redis::Client
}

impl Cache {
    pub async fn new(url: &str) -> Self {
        let client = redis::Client::open(url).unwrap();
        Cache { client: client }
    }

    /// Set a single user token. Overwrites.
    /// * `key` - user id
    /// * `value` - uuid
    pub async fn set_key(&self, key: &str, value: &str, expiry_sec: u64) -> Result<(), ()> {
        let mut conn = self.get_async_conn().await?;
        match conn.set_ex(key, value, expiry_sec).await {
            Ok(()) => Ok(()),
            Err(re) => {
                warn!("{}", re);
                Err(())
            }
        }
    }

    pub async fn set_single(&self, entry: UserToken, overwrite: bool) -> Result<(), ()> {
        let mut conn = self.get_async_conn().await?;

        match conn.set_ex(entry.user_id, entry.uuid.to_string(), entry.expiry_sec).await {
            Ok(()) => Ok(()),
            Err(re) => {
                warn!("{}", re);
                Err(())
            }
        }
    }

    /// Set multiple user user tokens using a Redis pipeline. Existing data
    /// is overwritten if the `overwrite` flag is set.
    pub async fn set_multiple(&self, entries: Vec<UserToken>, overwrite: bool) -> Result<(), ()> {
        let mut conn = self.get_async_conn().await?;

        let add_key_to_pipe = |entry: &UserToken, pipe: &mut redis::Pipeline| {
            // if let CacheEntry::UserToken { uuid, user_id, expiry_sec } = entry {
            if overwrite {
                pipe.set_ex(entry.user_id, entry.uuid.to_string(), entry.expiry_sec.clone())
                    .ignore();
            } else {
                // no set_nx with expiry method exists.
                pipe.cmd("SET").arg(entry.user_id).arg(entry.uuid.to_string())
                    .arg("NX").arg("EX").arg(entry.expiry_sec).ignore();
            }
            // }
        };

        let mut pipe = redis::pipe();
        entries.iter().for_each(|entry| add_key_to_pipe(entry, &mut pipe));
        let _ = pipe.query_async::<MultiplexedConnection, ()>(&mut conn).await;
        pipe.clear();

        Ok(())
    }

    pub async fn _clear_key(&self, key: &str) -> Result<(), ()> {
        let mut conn = self.get_async_conn().await?;

        match conn.del::<&str, u32>(key).await {
            Ok(1)   => Ok(()),
            Ok(_)   => Err(()),
            Err(re) => {
                warn!("{}", re);
                Err(())
            }
        }
    }

    pub async fn get_token_by_user_id(&self, key: u64) -> Result<Uuid, ()> {
        let mut conn = self.get_async_conn().await?;
        
        match conn.get::<&u64, u128>(&key).await {
            Ok(uuid) => Ok(Uuid::from_u128(uuid)),
            Err(_) => Err(())
        }
    }

    async fn get_async_conn(&self) -> Result<MultiplexedConnection, ()> {
        match self.client.get_multiplexed_async_connection().await {
            Ok(conn) => Ok(conn),
            Err(_) => Err(())
        }
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use redis::AsyncCommands;
    use uuid::Uuid;

    use crate::cache::cache::UserToken;

    use super::Cache;

    const SHORT_EXPIRY: u64 = 4;

    async fn test_context() -> Cache {
        dotenv::dotenv().ok();
        let cache_url = std::env::var("REDIS_DATABASE_URL").expect("REDIS_DATABASE_URL is not set");
        Cache::new(&cache_url).await
    }

    #[actix_web::test]
    async fn test_set_single() {
        let cache = test_context().await;
        let mut conn = cache.get_async_conn().await.unwrap();

        conn.del::<&str, u8>("!test_set_single!1");
        conn.del::<&str, u8>("!test_set_single!2");
        assert_eq!(Ok(()), cache.set_key("!test_set_single!1", "!test!1!", SHORT_EXPIRY).await);
        assert_eq!(Ok(()), cache.set_key("!test_set_single!2", "!test!2!", SHORT_EXPIRY).await);

        let test1 = conn.get::<&str, String>("!test_set_single!1").await;
        let test2 = conn.get::<&str, String>("!test_set_single!2").await;

        assert_eq!(true, test1.is_ok());
        assert_eq!(true, test2.is_ok());
        assert_eq!("!test!1!", test1.unwrap());
        assert_eq!("!test!2!", test2.unwrap());
    }

    #[actix_web::test]
    async fn test_set_keys_multiple_overwrite() {
        let cache = test_context().await;
        let mut conn = cache.get_async_conn().await.unwrap();

        let uuid_1 = Uuid::new_v4();
        let uuid_2 = Uuid::new_v4();
        let test_entry_1 = UserToken {
            uuid: uuid_1,
            user_id: 0,
            expiry_sec: SHORT_EXPIRY / 2
        };
        let test_entry_2 = UserToken {
            uuid: uuid_2,
            user_id: 1,
            expiry_sec: SHORT_EXPIRY / 2
        };

        let entries: Vec<UserToken> = vec![test_entry_1, test_entry_2];
        assert_eq!(Ok(()), cache.set_multiple(entries, true).await);
        
        let test_retrieve_1 = conn.get::<u64, String>(0).await;
        let test_retrieve_2 = conn.get::<u64, String>(1).await;
        let test_retrieve_1_exp = conn.ttl::<u64, u64>(0).await;
        let test_retrieve_2_exp = conn.ttl::<u64, u64>(1).await;
        assert!(test_retrieve_1.is_ok());
        assert!(test_retrieve_2.is_ok());

        // Stored UUID check
        let retrieved_uuid_1 = Uuid::from_str(&test_retrieve_1.unwrap());
        let retrieved_uuid_2 = Uuid::from_str(&test_retrieve_2.unwrap());
        assert!(retrieved_uuid_1.is_ok());
        assert!(retrieved_uuid_2.is_ok());
        assert_eq!(uuid_1, retrieved_uuid_1.unwrap());
        assert_eq!(uuid_2, retrieved_uuid_2.unwrap());

        // Expiry check
        assert_eq!(true, test_retrieve_1_exp.is_ok());
        assert_eq!(true, test_retrieve_2_exp.is_ok());
        assert!(test_retrieve_1_exp.unwrap() <= SHORT_EXPIRY / 2);
        assert!(test_retrieve_2_exp.unwrap() <= SHORT_EXPIRY / 2);

        let test_entry_1_altered = UserToken {
            uuid: uuid_1,
            user_id: 0,
            expiry_sec: SHORT_EXPIRY
        };

        let entries: Vec<UserToken> = vec![test_entry_1_altered];
        assert_eq!(Ok(()), cache.set_multiple(entries, true).await);
        let test_retrieve_1_alt = conn.get::<u64, String>(0).await;
        let test_retrieve_1_exp_alt = conn.ttl::<u64, u64>(0).await;
        assert!(test_retrieve_1_alt.is_ok());
        
        // Stored UUID check
        let retrieved_uuid_1_alt = Uuid::from_str(&test_retrieve_1_alt.unwrap());
        assert!(retrieved_uuid_1_alt.is_ok());
        assert_eq!(uuid_1, retrieved_uuid_1_alt.unwrap());
        
        // Updated expiry check
        assert!(test_retrieve_1_exp_alt.is_ok());
        assert!(test_retrieve_1_exp_alt.unwrap() > SHORT_EXPIRY / 2);
    }

    #[actix_web::test]
    async fn test_set_keys_multiple_no_overwrite() {
        let cache = test_context().await;
        let mut conn = cache.get_async_conn().await.unwrap();

        let uuid_1 = Uuid::new_v4();
        let uuid_2 = Uuid::new_v4();
        let test_entry_1 = UserToken {
            uuid: uuid_1,
            user_id: 2,
            expiry_sec: SHORT_EXPIRY / 2
        };
        let test_entry_2 = UserToken {
            uuid: uuid_2,
            user_id: 3,
            expiry_sec: SHORT_EXPIRY / 2
        };

        let entries: Vec<UserToken> = vec![test_entry_1, test_entry_2];
        assert_eq!(Ok(()), cache.set_multiple(entries, false).await);
        
        let test_retrieve_1 = conn.get::<u64, String>(2).await;
        let test_retrieve_2 = conn.get::<u64, String>(3).await;
        let test_retrieve_1_exp = conn.ttl::<u64, u64>(2).await;
        let test_retrieve_2_exp = conn.ttl::<u64, u64>(3).await;
        assert!(test_retrieve_1.is_ok());
        assert!(test_retrieve_2.is_ok());

        // Stored UUID check
        let retrieved_uuid_1 = Uuid::from_str(&test_retrieve_1.unwrap());
        let retrieved_uuid_2 = Uuid::from_str(&test_retrieve_2.unwrap());
        assert!(retrieved_uuid_1.is_ok());
        assert!(retrieved_uuid_2.is_ok());
        assert_eq!(uuid_1, retrieved_uuid_1.unwrap());
        assert_eq!(uuid_2, retrieved_uuid_2.unwrap());

        // Expiry check
        assert_eq!(true, test_retrieve_1_exp.is_ok());
        assert_eq!(true, test_retrieve_2_exp.is_ok());
        assert!(test_retrieve_1_exp.unwrap() <= SHORT_EXPIRY / 2);
        assert!(test_retrieve_2_exp.unwrap() <= SHORT_EXPIRY / 2);

        let test_entry_1_altered = UserToken {
            uuid: uuid_1,
            user_id: 0,
            expiry_sec: SHORT_EXPIRY
        };

        let entries: Vec<UserToken> = vec![test_entry_1_altered];
        assert_eq!(Ok(()), cache.set_multiple(entries, false).await);
        let test_retrieve_1_alt = conn.get::<u64, String>(2).await;
        let test_retrieve_1_exp_alt = conn.ttl::<u64, u64>(2).await;
        assert!(test_retrieve_1_alt.is_ok());
        
        // Stored UUID check
        let retrieved_uuid_1_alt = Uuid::from_str(&test_retrieve_1_alt.unwrap());
        assert!(retrieved_uuid_1_alt.is_ok());
        assert_eq!(uuid_1, retrieved_uuid_1_alt.unwrap());
        
        // Updated expiry check
        println!("{:?}", test_retrieve_1_exp_alt);
        // assert!(test_retrieve_1_exp_alt.is_ok_and(|expiry| expiry <= SHORT_EXPIRY / 2));
        assert!(test_retrieve_1_exp_alt.is_ok());
        assert!(test_retrieve_1_exp_alt.unwrap() <= SHORT_EXPIRY / 2);
    }
}
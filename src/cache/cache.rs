use log::warn;

use uuid::Uuid;

use redis::{aio::MultiplexedConnection, AsyncCommands, ConnectionLike, Pipeline};

pub struct UserToken {
    pub uuid: Uuid,
    pub user_id: u64,
    pub expiry_sec: u64
}

pub struct Cache {
    client: redis::Client
}

impl Cache {
    pub fn new(url: &str) -> Result<Self, ()> {
        let mut client = redis::Client::open(url).unwrap();
        match client.check_connection() {
            true  => Ok(Cache { client: client }),
            false => Err(())
        }
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

    /// Set a user tokens in the Redis DB.
    /// * `symmetric` - if true, makes two entries with the user ID
    ///                 and UUID pair in opposite Key:Value orders.
    /// * `overwrite` - if true, overwrites any existing entries.
    pub async fn set_single(
        &self,
        entry: UserToken,
        symmetric: bool,
        overwrite: bool
    ) -> Result<(), ()> {
        let mut conn = self.get_async_conn().await?;
        let mut pipe = redis::pipe();

        add_to_pipe(&mut pipe, &entry, symmetric, overwrite);

        let result = pipe.query_async::<MultiplexedConnection, ()>(&mut conn).await;
        pipe.clear();
        match result {
            Ok(_)   => Ok(()),
            Err(re) => {
                warn!("{}", re);
                Err(())
            },
        }
    }

    /// Set multiple user tokens in the Redis DB.
    /// * `symmetric` - if true, makes two entries with each user ID
    ///                 and UUID pair in opposite Key:Value orders.
    /// * `overwrite` - if true, overwrites any existing entries.
    pub async fn set_multiple(
        &self,
        entries: Vec<UserToken>,
        symmetric: bool,
        overwrite: bool,
    ) -> Result<(), ()> {
        let mut conn = self.get_async_conn().await?;

        let mut pipe = redis::pipe();

        entries.iter().for_each(|entry| add_to_pipe(&mut pipe, entry, symmetric, overwrite));

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
        
        match conn.get::<&u64, String>(&key).await {
            Ok(uuid) => Ok(Uuid::parse_str(&uuid).unwrap()),
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

fn add_to_pipe(pipe: &mut Pipeline, entry: &UserToken, symmetric: bool, overwrite: bool) -> () {
    let uuid = entry.uuid.to_string();
    let _ = match (symmetric, overwrite) {
        (true, true)   => pipe.set_ex(entry.user_id, &uuid, entry.expiry_sec)
                                .set_ex(uuid, entry.user_id, entry.expiry_sec)
                                .ignore(),
        (true, false)  => pipe.cmd("SET").arg(entry.user_id).arg(&uuid).arg("NX")
                                .arg("EX").arg(entry.expiry_sec).ignore()
                                .cmd("SET").arg(uuid).arg(entry.user_id).arg("NX")
                                .arg("EX").arg(entry.expiry_sec).ignore(),
        (false, true)  => pipe.set_ex(entry.user_id, uuid, entry.expiry_sec).ignore(),
        (false, false) => pipe.cmd("SET").arg(entry.user_id).arg(uuid).arg("NX").ignore()
                                .cmd("EXPIRE").arg(entry.user_id).arg(entry.expiry_sec)
                                .arg("NX").ignore()
    };
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use redis::AsyncCommands;
    use uuid::Uuid;

    use crate::cache::cache::UserToken;

    use super::Cache;

    const SHORT_EXPIRY: u64 = 4;

    fn test_context() -> Cache {
        dotenv::dotenv().ok();
        let cache_url = std::env::var("REDIS_DATABASE_URL").expect("REDIS_DATABASE_URL is not set");
        Cache::new(&cache_url).unwrap()
    }

    #[actix_web::test]
    async fn test_set_single() {
        let cache = test_context();
        let mut conn = cache.get_async_conn().await.unwrap();

        let _ = conn.del::<&str, u8>("!test_set_single!1").await;
        let _ = conn.del::<&str, u8>("!test_set_single!2").await;
        conn.del::<&str, u8>("!test_set_single!1").await;
        conn.del::<&str, u8>("!test_set_single!2").await;
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
    async fn test_set_single_symmetric_overwrite() {
        let cache = test_context();
        let mut conn = cache.get_async_conn().await.unwrap();

        let uuid = Uuid::new_v4();
        let entry = UserToken {
            uuid: uuid,
            user_id: 5,
            expiry_sec: SHORT_EXPIRY / 2
        };
        assert_eq!(Ok(()), cache.set_single(entry, true, true).await);

        let test_retrieve_by_id = conn.get::<u64, String>(5).await;
        let test_retrieve_by_id_exp = conn.ttl::<u64, u64>(5).await;
        let test_retrieve_by_uuid = conn.get::<String, u64>(uuid.to_string()).await;
        let test_retrieve_by_uuid_exp = conn.ttl::<String, u64>(uuid.to_string()).await;

        assert!(test_retrieve_by_id.is_ok(), "Failed to retrieve entry UUID by ID");
        assert!(test_retrieve_by_uuid.is_ok(), "Failed to retrieve entry ID by UUID");

        let retrieved_uuid = Uuid::from_str(&test_retrieve_by_id.unwrap());
        assert!(retrieved_uuid.is_ok(), "Retrieved UUID (by ID) failed to parse");
        assert_eq!(uuid, retrieved_uuid.unwrap(), "Retrieved UUID (by ID) is invalid");

        assert_eq!(5, test_retrieve_by_uuid.unwrap());

        assert!(test_retrieve_by_id_exp.is_ok(), "Failed to retrieve TTL of entry (id as key)");
        assert!(test_retrieve_by_uuid_exp.is_ok(), "Failed to retrieve TTL of entry (uuid as key)");
        assert!(test_retrieve_by_id_exp.unwrap() <= SHORT_EXPIRY / 2);
        assert!(test_retrieve_by_uuid_exp.unwrap() <= SHORT_EXPIRY / 2);

        // Overwrite

        let updated_expiry = UserToken {
            uuid: uuid,
            user_id: 5,
            expiry_sec: SHORT_EXPIRY
        };
        assert_eq!(Ok(()), cache.set_single(updated_expiry, true, true).await);

        let test_retrieve_updated_by_id = conn.get::<u64, String>(5).await;
        let test_retrieve_updated_by_id_exp = conn.ttl::<u64, u64>(5).await;
        let test_retrieve_updated_by_uuid = conn.get::<String, u64>(uuid.to_string()).await;
        let test_retrieve_updated_by_uuid_exp = conn.ttl::<String, u64>(uuid.to_string()).await;

        assert!(test_retrieve_updated_by_id.is_ok(), "Failed to retrieve updated entry by ID");
        assert!(test_retrieve_updated_by_uuid.is_ok(), "Failed to retrieve updated entry by UUID");

        let retrieved_updated_uuid = Uuid::from_str(&test_retrieve_updated_by_id.unwrap());
        assert!(retrieved_updated_uuid.is_ok(), "Retrieved UUID (by ID) failed to parse after update");
        assert_eq!(uuid, retrieved_updated_uuid.unwrap(), "Retrieved UUID is invalid after update");

        assert_eq!(5, test_retrieve_updated_by_uuid.unwrap());

        assert!(test_retrieve_updated_by_id_exp.is_ok(), "Failed to retrieve updated TTL (id as key)");
        assert!(test_retrieve_updated_by_uuid_exp.is_ok(), "Failed to retrieve updated TTL (uuid as key)");
        assert!(test_retrieve_updated_by_id_exp.unwrap() > SHORT_EXPIRY / 2);
        assert!(test_retrieve_updated_by_uuid_exp.unwrap() > SHORT_EXPIRY / 2);
    }

    #[actix_web::test]
    async fn test_set_multiple_asymmetric_overwrite() {
        let cache = test_context();
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
        assert_eq!(Ok(()), cache.set_multiple(entries, false, true).await);
        
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
        assert_eq!(Ok(()), cache.set_multiple(entries, false, true).await);
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
    async fn test_set_multiple_asymmetric_no_overwrite() {
        let cache = test_context();
        let mut conn = cache.get_async_conn().await.unwrap();

        let _ = conn.del::<u64, String>(2).await;
        let _ = conn.del::<u64, String>(3).await;

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
        assert_eq!(Ok(()), cache.set_multiple(entries, false, false).await);
        
        let test_retrieve_1 = conn.get::<u64, String>(2).await;
        let test_retrieve_2 = conn.get::<u64, String>(3).await;
        let test_retrieve_1_exp = conn.ttl::<u64, u64>(2).await;
        let test_retrieve_2_exp = conn.ttl::<u64, u64>(3).await;
        assert!(test_retrieve_1.is_ok());
        assert!(test_retrieve_2.is_ok());

        // Stored UUID check
        let retrieved_uuid_1 = Uuid::from_str(&test_retrieve_1.unwrap());
        let retrieved_uuid_2 = Uuid::from_str(&test_retrieve_2.unwrap());
        assert!(retrieved_uuid_1.is_ok(), "entry 1 was not retrieved");
        assert!(retrieved_uuid_2.is_ok(), "entry 2 was not retrieved");
        assert_eq!(uuid_1, retrieved_uuid_1.unwrap(), "entry 1 uuid did not match");
        assert_eq!(uuid_2, retrieved_uuid_2.unwrap(), "entry 2 uuid did not match");

        // Expiry check
        assert_eq!(true, test_retrieve_1_exp.is_ok(), "entry 1 TTL/expiry was not retrieved");
        assert_eq!(true, test_retrieve_2_exp.is_ok(), "entry 2 TTL/expiry was not retrieved");
        assert!(test_retrieve_1_exp.unwrap() <= SHORT_EXPIRY / 2);
        assert!(test_retrieve_2_exp.unwrap() <= SHORT_EXPIRY / 2);

        let test_entry_1_altered = UserToken {
            uuid: uuid_1,
            user_id: 2,
            expiry_sec: SHORT_EXPIRY
        };

        let entries: Vec<UserToken> = vec![test_entry_1_altered];
        assert_eq!(Ok(()), cache.set_multiple(entries, false, false).await);
        let test_retrieve_1_alt = conn.get::<u64, String>(2).await;
        let test_retrieve_1_exp_alt = conn.ttl::<u64, u64>(2).await;
        assert!(test_retrieve_1_alt.is_ok(), "entry 1 was not retrieved after second set");
        
        // Stored UUID check
        let retrieved_uuid_1_alt = Uuid::from_str(&test_retrieve_1_alt.unwrap());
        assert!(retrieved_uuid_1_alt.is_ok(), "entry 1 uuid after second set could not be parsed");
        assert_eq!(uuid_1, retrieved_uuid_1_alt.unwrap(), "entry 1 uuid after second set invalid");
        
        // Updated expiry check
        println!("{:?}", test_retrieve_1_exp_alt);
        // assert!(test_retrieve_1_exp_alt.is_ok_and(|expiry| expiry <= SHORT_EXPIRY / 2));
        assert!(test_retrieve_1_exp_alt.is_ok(), "entry 1 expiry after second set was not retrieved");
        assert!(test_retrieve_1_exp_alt.unwrap() <= SHORT_EXPIRY / 2, "entry 1 expiry was updated");
    }
}
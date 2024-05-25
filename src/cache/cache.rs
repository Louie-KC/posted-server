use log::warn;

use uuid::Uuid;

use redis::{aio::MultiplexedConnection, AsyncCommands, ConnectionLike, Pipeline};

use super::error::CacheErr;

pub struct Entry {
    pub key: String,
    pub value: String,
    pub expiry_sec: u64
}

impl Entry {
    pub fn new(key: String, value: String, expiry_sec: u64) -> Self {
        Entry { key, value, expiry_sec }
    }
}

trait RedisPipelineExt {
    fn set_ex_symmetric(&mut self, entry: &Entry) -> &mut Self;
    fn set_ex_nx(&mut self, entry: &Entry) -> &mut Self;
    fn set_ex_nx_symmetric(&mut self, entry: &Entry) -> &mut Self;
}

impl RedisPipelineExt for redis::Pipeline {
    fn set_ex_symmetric(&mut self, entry: &Entry) -> &mut Self {
        self.cmd("SET").arg(&entry.key).arg(&entry.value)
            .arg("EX").arg(entry.expiry_sec)
            .cmd("SET").arg(&entry.value).arg(&entry.key)
            .arg("EX").arg(entry.expiry_sec)
    }

    fn set_ex_nx(&mut self, entry: &Entry) -> &mut Self {
        self.cmd("SET").arg(&entry.key).arg(&entry.value)
            .arg("EX").arg(entry.expiry_sec).arg("NX")
    }

    fn set_ex_nx_symmetric(&mut self, entry: &Entry) -> &mut Self {
        self.cmd("SET").arg(&entry.key).arg(&entry.value)
            .arg("EX").arg(entry.expiry_sec).arg("NX")
            .cmd("SET").arg(&entry.value).arg(&entry.key)
            .arg("EX").arg(entry.expiry_sec).arg("NX")
    }
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

    pub async fn get(&self, key: &str) -> Result<String, CacheErr> {
        let mut conn = match self.get_async_conn().await {
            Ok(conn) => conn,
            Err(_) => return Err(CacheErr::AsyncConnFailure),
        };
        match conn.get(key).await {
            Ok(value) => Ok(value),
            Err(re) => Err(CacheErr::from(re))
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

    /// Set an entry in the Redis DB.
    /// * `symmetric` - if true, makes two entries using the provided
    ///                 `entry`, where the extra has the key-value swapped.
    /// * `overwrite` - if true, overwrites any existing entries.
    pub async fn set_single(
        &self,
        entry: Entry,
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
    /// * `symmetric` - if true, makes two entries using the provided
    ///                 `entry`, where the extra has the key-value swapped.
    /// * `overwrite` - if true, overwrites any existing entries.
    pub async fn set_multiple(
        &self,
        entries: Vec<Entry>,
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

fn add_to_pipe(pipe: &mut Pipeline, entry: &Entry, symmetric: bool, overwrite: bool) -> () {
    match (symmetric, overwrite) {
        (true, true)   => pipe.set_ex_symmetric(entry),
        (true, false)  => pipe.set_ex_nx_symmetric(entry),
        (false, true)  => pipe.set_ex(entry.key.clone(), entry.value.clone(), entry.expiry_sec),
        (false, false) => pipe.set_ex_nx(entry),
    };
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use redis::AsyncCommands;
    use uuid::Uuid;

    use crate::cache::cache::Entry;

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
        let _ = conn.del::<&str, u8>("!test_set_single!1").await;
        let _ = conn.del::<&str, u8>("!test_set_single!2").await;
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
        let user_id = 5;

        let entry = Entry {
            key: uuid.to_string(),
            value: user_id.to_string(),
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

        let updated_expiry = Entry {
            key: uuid.to_string(),
            value: user_id.to_string(),
            expiry_sec: SHORT_EXPIRY
        };
        assert_eq!(Ok(()), cache.set_single(updated_expiry, true, true).await);

        let test_retrieve_updated_by_id = conn.get::<u64, String>(user_id).await;
        let test_retrieve_updated_by_id_exp = conn.ttl::<u64, u64>(user_id).await;
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
        let user_id_1 = 0;
        let user_id_2 = 1;

        let test_entry_1 = Entry {
            key: uuid_1.to_string(),
            value: user_id_1.to_string(),
            expiry_sec: SHORT_EXPIRY / 2
        };
        let test_entry_2 = Entry {
            key: uuid_2.to_string(),
            value: user_id_2.to_string(),
            expiry_sec: SHORT_EXPIRY / 2
        };

        let entries: Vec<Entry> = vec![test_entry_1, test_entry_2];
        assert_eq!(Ok(()), cache.set_multiple(entries, false, true).await);
        
        let test_retrieve_1 = conn.get::<String, u64>(uuid_1.to_string()).await;
        let test_retrieve_2 = conn.get::<String, u64>(uuid_2.to_string()).await;
        let test_retrieve_1_exp = conn.ttl::<String, u64>(uuid_1.to_string()).await;
        let test_retrieve_2_exp = conn.ttl::<String, u64>(uuid_2.to_string()).await;
        assert!(test_retrieve_1.is_ok());
        assert!(test_retrieve_2.is_ok());

        // Ensure stored values are correct
        assert_eq!(user_id_1, test_retrieve_1.unwrap(), "stored entry 1 user id was invalid");
        assert_eq!(user_id_2, test_retrieve_2.unwrap(), "stored entry 2 user id was invalid");

        // Expiry check
        assert_eq!(true, test_retrieve_1_exp.is_ok());
        assert_eq!(true, test_retrieve_2_exp.is_ok());
        assert!(test_retrieve_1_exp.unwrap() <= SHORT_EXPIRY / 2);
        assert!(test_retrieve_2_exp.unwrap() <= SHORT_EXPIRY / 2);

        let test_entry_1_altered = Entry {
            key: uuid_1.to_string(),
            value: user_id_1.to_string(),
            expiry_sec: SHORT_EXPIRY
        };

        let entries: Vec<Entry> = vec![test_entry_1_altered];
        assert_eq!(Ok(()), cache.set_multiple(entries, false, true).await);
        let test_retrieve_1_alt = conn.get::<String, u64>(uuid_1.to_string()).await;
        let test_retrieve_1_exp_alt = conn.ttl::<String, u64>(uuid_1.to_string()).await;
        assert!(test_retrieve_1_alt.is_ok());
        
        // Stored value re-check
        assert_eq!(user_id_1, test_retrieve_1_alt.unwrap(), "stored user id for entry 1 changed");
        
        // Updated expiry check
        assert!(test_retrieve_1_exp_alt.is_ok());
        assert!(test_retrieve_1_exp_alt.unwrap() > SHORT_EXPIRY / 2);
    }

    #[actix_web::test]
    async fn test_set_multiple_asymmetric_no_overwrite() {
        let cache = test_context();
        let mut conn = cache.get_async_conn().await.unwrap();

        let uuid_1 = Uuid::new_v4();
        let uuid_2 = Uuid::new_v4();
        let user_id_1 = 2;
        let user_id_2 = 3;

        let test_entry_1 = Entry {
            key: uuid_1.to_string(),
            value: user_id_1.to_string(),
            expiry_sec: SHORT_EXPIRY / 2
        };
        let test_entry_2 = Entry {
            key: uuid_2.to_string(),
            value: user_id_2.to_string(),
            expiry_sec: SHORT_EXPIRY / 2
        };

        let entries: Vec<Entry> = vec![test_entry_1, test_entry_2];
        assert_eq!(Ok(()), cache.set_multiple(entries, false, false).await);
        
        let test_retrieve_1 = conn.get::<String, u64>(uuid_1.to_string()).await;
        let test_retrieve_2 = conn.get::<String, u64>(uuid_2.to_string()).await;
        let test_retrieve_1_exp = conn.ttl::<String, u64>(uuid_1.to_string()).await;
        let test_retrieve_2_exp = conn.ttl::<String, u64>(uuid_2.to_string()).await;
        assert!(test_retrieve_1.is_ok());
        assert!(test_retrieve_2.is_ok());

        // Stored user id check
        assert_eq!(user_id_1, test_retrieve_1.unwrap(), "stored entry 1 user id was invalid");
        assert_eq!(user_id_2, test_retrieve_2.unwrap(), "stored entry 2 user id was invalid");

        // Expiry check
        assert_eq!(true, test_retrieve_1_exp.is_ok(), "entry 1 TTL/expiry was not retrieved");
        assert_eq!(true, test_retrieve_2_exp.is_ok(), "entry 2 TTL/expiry was not retrieved");
        assert!(test_retrieve_1_exp.unwrap() <= SHORT_EXPIRY / 2);
        assert!(test_retrieve_2_exp.unwrap() <= SHORT_EXPIRY / 2);

        let test_entry_1_altered = Entry {
            key: uuid_1.to_string(),
            value: user_id_1.to_string(),
            expiry_sec: SHORT_EXPIRY
        };

        let entries: Vec<Entry> = vec![test_entry_1_altered];
        assert_eq!(Ok(()), cache.set_multiple(entries, false, false).await);
        let test_retrieve_1_alt = conn.get::<String, u64>(uuid_1.to_string()).await;
        let test_retrieve_1_exp_alt = conn.ttl::<String, u64>(uuid_1.to_string()).await;
        assert!(test_retrieve_1_alt.is_ok(), "entry 1 was not retrieved after second set");
        
        // Stored user id 1 re-check
        assert_eq!(user_id_1, test_retrieve_1_alt.unwrap(), "The stored user id 1 has changed");
        
        // Updated expiry check
        assert!(test_retrieve_1_exp_alt.is_ok(), "entry 1 expiry after second set was not retrieved");
        assert!(test_retrieve_1_exp_alt.unwrap() <= SHORT_EXPIRY / 2, "entry 1 expiry was updated");
    }
}
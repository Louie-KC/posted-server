use log::warn;

use uuid::Uuid;

use redis::{aio::MultiplexedConnection, AsyncCommands};

pub struct Cache {
    client: redis::Client
}

impl Cache {
    pub async fn new(url: &str) -> Self {
        let client = redis::Client::open(url).unwrap();
        Cache { client: client }
    }

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
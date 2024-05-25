use redis::RedisError;

pub enum CacheErr {
    NilResponse,
    AsyncConnFailure,
    ConnectionLost,
    RedisErr(RedisError),
}

impl From<RedisError> for CacheErr {
    fn from(value: RedisError) -> Self {
        if value.is_connection_dropped() || value.is_unrecoverable_error() {
            return CacheErr::ConnectionLost
        }
        match value.kind() {
            redis::ErrorKind::TypeError => CacheErr::NilResponse,
            _ => CacheErr::RedisErr(value),
        }
    }
}
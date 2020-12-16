use crate::{WebResult, Result, REDIS_CON_STRING};
use mobc::{Connection, Pool};
use mobc_redis::redis::{AsyncCommands, FromRedisValue};
use mobc_redis::{redis, RedisConnectionManager};
use std::time::Duration;
use std::convert::Infallible;
use warp::{Filter, Reply};

use crate::enums::{MobcError::*};

pub type MobcPool = Pool<RedisConnectionManager>;
pub type MobcCon = Connection<RedisConnectionManager>;

const CACHE_POOL_MAX_OPEN: u64 = 16;
const CACHE_POOL_MAX_IDLE: u64 = 8;
const CACHE_POOL_TIMEOUT_SECONDS: u64 = 1;
const CACHE_POOL_EXPIRE_SECONDS: u64 = 60;

pub async fn connect() -> Result<MobcPool> {
    let client = redis::Client::open(REDIS_CON_STRING).map_err(RedisClientError)?;
    let manager = RedisConnectionManager::new(client);
    Ok(Pool::builder()
        .get_timeout(Some(Duration::from_secs(CACHE_POOL_TIMEOUT_SECONDS)))
        .max_open(CACHE_POOL_MAX_OPEN)
        .max_idle(CACHE_POOL_MAX_IDLE)
        .max_lifetime(Some(Duration::from_secs(CACHE_POOL_EXPIRE_SECONDS)))
        .build(manager))
}

pub async fn get_con(pool: &MobcPool) -> Result<MobcCon> {
    pool.get().await.map_err(|e| {
        std::eprintln!("error connecting to redis: {}", e);
        RedisPoolError(e).into()
    })
}

pub async fn set_str(pool: &MobcPool, key: &str, value: &str, ttl_seconds: usize) -> Result<()> {
    let mut con = get_con(&pool).await?;
    con.set(key, value).await.map_err(RedisCMDError)?;
    if ttl_seconds > 0 {
        con.expire(key, ttl_seconds).await.map_err(RedisCMDError)?;
    }
    Ok(())
}

pub async fn get_str(pool: &MobcPool, key: &str) -> Result<String> {
    let mut con = get_con(&pool).await?;
    let value = con.get(key).await.map_err(RedisCMDError)?;
    FromRedisValue::from_redis_value(&value).map_err(|e| RedisTypeError(e).into())
}

pub async fn handler(pool: MobcPool) -> WebResult<impl Reply> {
    set_str(&pool, "mobc_hello", "mobc_world", 60 as usize)
        .await
        .map_err(|e| warp::reject::custom(e))?;
    let value = get_str(&pool, "mobc_hello")
        .await
        .map_err(|e| warp::reject::custom(e))?;
    Ok(value)
}

pub fn with_mobc_pool(
    pool: MobcPool,
) -> impl Filter<Extract = (MobcPool,), Error = Infallible> + Clone {
    warp::any().map(move || pool.clone())
}

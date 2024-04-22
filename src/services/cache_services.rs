use anyhow::Result;
use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use serde::{de::DeserializeOwned, Serialize};

pub async fn add_cached_value<T: Serialize>(
    pool: &Pool<RedisConnectionManager>,
    cache_field: &str,
    field_id: &str,
    cache_value: &T,
) -> Result<()> {
    let study_json = serde_json::to_string(cache_value)?;
    let mut conn = pool.get().await?;
    redis::cmd("HSET")
        .arg(cache_field)
        .arg(field_id)
        .arg(study_json)
        .query_async(&mut *conn)
        .await?;

    Ok(())
}

pub async fn delete_cached_value(
    pool: &Pool<RedisConnectionManager>,
    cache_field: &str,
    field_id: &str,
) -> Result<()> {
    let mut conn = pool.get().await?;
    redis::cmd("DEL")
        .arg(cache_field)
        .arg(field_id)
        .query_async(&mut *conn)
        .await?;

    Ok(())
}

pub async fn get_cached_value<T: DeserializeOwned>(
    pool: &Pool<RedisConnectionManager>,
    cache_field: &str,
    field_id: &str,
) -> Result<Option<T>> {
    let mut conn = pool.get().await?;
    let cached_study_str: Option<String> = redis::cmd("HGET")
        .arg(cache_field)
        .arg(field_id)
        .query_async(&mut *conn)
        .await?;

    match cached_study_str {
        Some(c) => {
            let cached_value: T = serde_json::from_str(&c)?;
            Ok(Some(cached_value))
        }
        None => Ok(None),
    }
}

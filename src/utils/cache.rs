use deadpool_redis::{Pool, redis::AsyncCommands};
use serde::{Serialize, de::DeserializeOwned};
use std::time::Duration;

#[derive(Clone)]
pub struct CacheService {
    pub pool: Pool,
    pub default_ttl: Duration,
}

impl CacheService {
    pub fn new(pool: Pool) -> Self {
        Self {
            pool,
            default_ttl: Duration::from_secs(60),
        }
    }

    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, String> {
        let mut conn = self.pool.get().await.map_err(|e| e.to_string())?;
        let value: Option<String> = conn.get(key).await.map_err(|e| e.to_string())?;

        match value {
            Some(json_str) => serde_json::from_str(&json_str).map_err(|e| e.to_string()),
            None => Ok(None),
        }
    }

    pub async fn set<T: Serialize>(
        &self,
        key: &str,
        value: &T,
        ttl: Option<Duration>,
    ) -> Result<(), String> {
        let mut conn = self.pool.get().await.map_err(|e| e.to_string())?;
        let json_str = serde_json::to_string(value).map_err(|e| e.to_string())?;
        let ttl_seconds = ttl.unwrap_or(self.default_ttl).as_secs();

        let _: () = conn
            .set_ex(key, json_str, ttl_seconds)
            .await
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    pub async fn delete(&self, key: &str) -> Result<(), String> {
        let mut conn = self.pool.get().await.map_err(|e| e.to_string())?;
        let _: () = conn.del(key).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn invalidate_pattern(&self, pattern: &str) -> Result<(), String> {
        let mut conn = self.pool.get().await.map_err(|e| e.to_string())?;
        let keys: Vec<String> = conn.keys(pattern).await.map_err(|e| e.to_string())?;
        if !keys.is_empty() {
            let _: () = conn.del(keys).await.map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

pub mod keys {
    use uuid::Uuid;

    pub fn user_profile(user_id: Uuid) -> String {
        format!("user:profile:{}", user_id)
    }

    pub fn user_posts(user_id: Uuid, page: u32, limit: u32) -> String {
        format!("user:posts:{}:p{}:l{}", user_id, page, limit)
    }

    pub fn post(post_id: Uuid) -> String {
        format!("post:{}", post_id)
    }

    pub fn feed(user_id: Uuid, page: u32, limit: u32) -> String {
        format!("feed:{}:p{}:l{}", user_id, page, limit)
    }

    pub fn comments(post_id: Uuid, page: u32, limit: u32) -> String {
        format!("comments:{}:p{}:l{}", post_id, page, limit)
    }

    pub fn likes(post_id: Uuid) -> String {
        format!("likes:{}", post_id)
    }

    pub fn followers(user_id: Uuid, page: u32, limit: u32) -> String {
        format!("followers:{}:p{}:l{}", user_id, page, limit)
    }

    pub fn following(user_id: Uuid, page: u32, limit: u32) -> String {
        format!("following:{}:p{}:l{}", user_id, page, limit)
    }

    pub fn user(user_id: Uuid) -> String {
        format!("user:{}", user_id)
    }

    pub fn user_by_username(username: &str) -> String {
        format!("user:username:{}", username)
    }

    pub fn user_profile_with_posts(user_id: Uuid, page: u32, limit: u32) -> String {
        format!("user:profile:{}:p{}:l{}", user_id, page, limit)
    }

    pub fn post_likes(post_id: Uuid) -> String {
        format!("likes:count:{}", post_id)
    }

    pub fn user_like_status(user_id: Uuid, post_id: Uuid) -> String {
        format!("likes:status:{}:{}", user_id, post_id)
    }

    pub fn post_comments(post_id: Uuid, page: u32, limit: u32) -> String {
        format!("comments:post:{}:p{}:l{}", post_id, page, limit)
    }

    pub fn user_liked_post(user_id: Uuid, post_id: Uuid) -> String {
        format!("user:{}:liked:{}", user_id, post_id)
    }

    pub fn follower_count(user_id: Uuid) -> String {
        format!("followers:count:{}", user_id)
    }

    pub fn following_count(user_id: Uuid) -> String {
        format!("following:count:{}", user_id)
    }
}

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub name: Option<String>,
    pub password_hash: String,
    pub birthday: Option<chrono::NaiveDate>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct RegisterUser {
    pub username: String,
    pub email: String,
    pub name: Option<String>,
    pub password: String,
    pub birthday: Option<chrono::NaiveDate>,
}

#[derive(Debug, Deserialize)]
pub struct LoginUser {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUser {
    pub username: Option<String>,
    pub email: Option<String>,
    pub name: Option<String>,
    pub password: Option<String>,
    pub birthday: Option<chrono::NaiveDate>,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub name: Option<String>,
    pub birthday: Option<chrono::NaiveDate>,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        UserResponse {
            id: user.id,
            username: user.username,
            email: user.email,
            name: user.name,
            birthday: user.birthday,
            created_at: user.created_at,
        }
    }
}

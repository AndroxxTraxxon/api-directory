use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;


#[derive(Serialize, Deserialize, Clone, Debug, Validate)]
pub struct User {
    #[validate(length(min=4))]
    pub username: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
    pub password_reset_at: Option<DateTime<Utc>>, // Field to store the datetime of the last password reset
    pub scopes: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Validate)]
pub struct AuthUser {
    pub username: String,
    pub scopes: Vec<String>
}
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;
use validator::Validate;


#[derive(Serialize, Deserialize, Clone, Debug, Validate)]
pub struct GatewayUser {
    pub id: Option<Thing>,
    #[validate(length(min=4))]
    pub username: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
    pub password_reset_at: Option<DateTime<Utc>>, // Field to store the datetime of the last password reset
    pub scopes: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Validate)]
pub struct PasswordResetRequest {
    pub id: Option<Thing>,
    pub user_id: String,
    pub used: bool,
    pub expires_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Validate)]
pub struct PartialGatewayUserUpdate {
    #[validate(length(min=4))]
    pub username: Option<String>,
    pub scopes: Option<Vec<String>>,
}
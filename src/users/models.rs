use serde::{Deserialize, Serialize};
use surrealdb::sql::{Thing, Datetime};
use validator::Validate;

#[derive(Serialize, Deserialize, Clone, Debug, Validate)]
pub struct GatewayUser {
    pub id: Option<Thing>,
    #[validate(length(min = 4))]
    pub username: String,
    pub scopes: Vec<String>,
    pub created_date: Datetime,
    pub last_modified_date: Datetime,
    pub last_login: Option<Datetime>,
    pub password_reset_at: Option<Datetime>, // Field to store the datetime of the last password reset
}

#[derive(Serialize, Deserialize, Clone, Debug, Validate)]
pub struct PartialGatewayUserUpdate {
    #[validate(length(min = 4))]
    pub username: Option<String>,
    pub scopes: Option<Vec<String>>,
}

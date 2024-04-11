use crate::api_services::models::{DbApiRole, WebApiRole};
use serde::{Deserialize, Serialize};
use surrealdb::sql::{Datetime, Thing};
use validator::Validate;

#[derive(Serialize, Deserialize, Clone, Debug, Validate)]
pub struct DbGatewayUserRecord {
    pub id: Thing,
    #[validate(length(min = 4))]
    pub username: String,
    pub created_date: Datetime,
    pub last_modified_date: Datetime,
    pub last_login: Option<Datetime>,
    pub password_reset_at: Option<Datetime>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Validate)]
pub struct DbGatewayUserResponse {
    pub id: Thing,
    #[validate(length(min = 4))]
    pub username: String,
    pub roles: Vec<DbApiRole>,
    pub created_date: Datetime,
    pub last_modified_date: Datetime,
    pub last_login: Option<Datetime>,
    pub password_reset_at: Option<Datetime>, // Field to store the datetime of the last password reset
}

#[derive(Serialize, Deserialize, Clone, Debug, Validate)]
pub struct WebGatewayUserResponse {
    pub id: String,
    #[validate(length(min = 4))]
    pub username: String,
    pub roles: Vec<WebApiRole>,
    pub created_date: Datetime,
    pub last_modified_date: Datetime,
    pub last_login: Option<Datetime>,
    pub password_reset_at: Option<Datetime>, // Field to store the datetime of the last password reset
}

#[derive(Serialize, Deserialize, Clone, Debug, Validate)]
pub struct WebPartialGatewayUserUpdate {
    #[validate(length(min = 4))]
    pub username: Option<String>,
    pub roles: Option<Vec<WebApiRole>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Validate)]
pub struct DbPartialGatewayUserUpdate {
    #[validate(length(min = 4))]
    pub username: Option<String>,
}

impl From<&WebPartialGatewayUserUpdate> for DbPartialGatewayUserUpdate {
    fn from(value: &WebPartialGatewayUserUpdate) -> Self {
        Self {
            username: value.username.clone(),
        }
    }
}

impl From<&WebPartialGatewayUserUpdate> for Option<Vec<DbApiRole>> {
    fn from(value: &WebPartialGatewayUserUpdate) -> Self {
        match &value.roles {
            Some(roles) => Some(roles.iter().map(DbApiRole::from).collect()),
            None => None
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Validate)]

pub struct WebGatewayUserRequest {
    #[validate(length(min = 4))]
    pub username: String,
    pub roles: Vec<WebApiRole>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DbGatewayUserRequest {
    pub username: String,
}

impl From<&WebGatewayUserRequest> for DbGatewayUserRequest {
    fn from(value: &WebGatewayUserRequest) -> Self {
        Self {
            username: value.username.clone(),
        }
    }
}

impl From<&WebGatewayUserRequest> for Vec<DbApiRole> {
    fn from(value: &WebGatewayUserRequest) -> Self {
        value.roles.iter().map(|role| role.into()).collect()
    }
}


impl From<&DbGatewayUserResponse> for WebGatewayUserResponse {
    fn from(value: &DbGatewayUserResponse) -> Self {
        Self {
            id: format!("{}", value.id.id),
            username: value.username.clone(),
            roles: value.roles.iter().map(|role| role.into()).collect(),
            created_date: value.created_date.clone(),
            last_modified_date: value.last_modified_date.clone(),
            last_login: value.last_login.clone(),
            password_reset_at: value.password_reset_at.clone(),
        }
    }
}

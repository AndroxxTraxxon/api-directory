use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize, Serializer};
use surrealdb::sql::Thing;
use validator::Validate;

#[derive(Serialize, Deserialize, Clone, Debug, Validate)]
pub struct GatewayUser {
    pub id: Option<Thing>,
    #[validate(length(min = 4))]
    pub username: String,
    pub password_hash: String,
    pub scopes: Vec<String>,
    #[serde(serialize_with="serialize_chrono_as_sql_datetime")]
    pub created_date: DateTime<Utc>,
    #[serde(serialize_with="serialize_chrono_as_sql_datetime")]
    pub last_modified_date: DateTime<Utc>,
    #[serde(serialize_with="serialize_chrono_option_as_sql_datetime")]
    pub last_login: Option<DateTime<Utc>>,
    #[serde(serialize_with="serialize_chrono_option_as_sql_datetime")]
    pub password_reset_at: Option<DateTime<Utc>>, // Field to store the datetime of the last password reset
}

pub fn serialize_chrono_as_sql_datetime<S>(
    x: &chrono::DateTime<Utc>,
    s: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    Into::<surrealdb::sql::Datetime>::into(*x).serialize(s)
}

pub fn serialize_chrono_option_as_sql_datetime<S>(
    x: &Option<chrono::DateTime<Utc>>,
    s: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match x {
        Some(value) => Into::<surrealdb::sql::Datetime>::into(*value).serialize(s),
        None => s.serialize_none()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Validate)]
pub struct PartialGatewayUserUpdate {
    #[validate(length(min = 4))]
    pub username: Option<String>,
    pub scopes: Option<Vec<String>>,
}

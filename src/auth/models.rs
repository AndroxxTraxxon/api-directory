use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey};
use serde::{Deserialize, Serialize};
use surrealdb::sql::{Datetime, Thing};
use validator::Validate;
// Define a struct to represent your user record
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GatewayUserClaims {
    // Using standard JWT properties:
    // https://www.iana.org/assignments/jwt/jwt.xhtml#claims
    // Issuer (us/apigateway.local)
    pub iss: String,
    // Subject (username)
    pub sub: String,
    // Subject ID (user table Id)
    pub sub_id: String,
    // Audience (for us, scopes)
    pub aud: Vec<String>,
    // Expires at (datetime-formatted string)
    pub exp: u64,
    // Issued at (datetime-formatted String)
    pub iat: u64,
    // Not before (datetime-formatted string)
    pub nbf: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GatewayLoginCredentials {
    pub username: String,
    pub password: String,
}

#[derive(Clone)]
pub struct JwtConfig {
    pub decoding_key: DecodingKey,
    pub encoding_key: EncodingKey,
    pub issuer: String,
    pub algorithm: Algorithm,
}

#[derive(Serialize, Deserialize, Clone, Debug, Validate)]
pub struct PasswordResetRequest {
    pub id: Option<Thing>,
    pub user_id: String,
    pub used: bool,
    pub expires_at: u64,
    pub last_modified: Datetime,
}

#[derive(Deserialize)]
pub struct PasswordForm {
    pub password: String,
}

#[derive(Deserialize)]
pub struct UserForm {
    pub username: String,
}

#[derive(Deserialize)]
pub struct RequestIdParams {
    pub request_id: String,
}

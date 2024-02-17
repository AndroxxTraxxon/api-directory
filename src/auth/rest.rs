use super::{models::{GatewayLoginCredentials, JwtConfig}, repo::UserAuthRepository};
use crate::errors::{GatewayError, Result};
use crate::database::Database;
use surrealdb::sql::{Id, Thing};
use actix_web::{
    post,
    web::{scope, Data, Json, ServiceConfig},
    HttpRequest
};
use std::time::SystemTime;
use jsonwebtoken::{decode, encode, Validation, Header};
use super::models::GatewayUserClaims;

const GATEWAY_JWT_ISSUER: &str = "apigateway.local";

// Intermediate function to configure services
pub fn web_setup(cfg: &mut ServiceConfig) {
    cfg.service(scope("/auth/v1").service(authenticate_user));
}

#[post("/login")]
async fn authenticate_user(
    req: HttpRequest, 
    repo: Data<Database>,
    credential_form: Json<GatewayLoginCredentials>,
) -> Result<String> {
    let credentials = credential_form.into_inner();
    let user = Database::authenticate_user(&repo, &credentials.username, &credentials.password)
        .await?;
    if let Some(Thing{tb: _, id: Id::String(user_id)}) = user.id {
        let now_ts = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
        let duration = 24 * 60 * 60;
        let claims = GatewayUserClaims {
            // Issuer (us/apigateway.local)
            iss: String::from(GATEWAY_JWT_ISSUER),
            sub: user.username,
            sub_id: user_id,
            aud: user.scopes,
            exp: now_ts + duration,
            iat: now_ts,
            nbf: now_ts,
        };
        let config: &Data<JwtConfig> = &req.app_data().unwrap();
        encode(&Header::default(), &claims, &config.encoding_key)
            .map_err(|e| GatewayError::TokenEncodeError(e.to_string()))
    } else {
        Err(GatewayError::TokenEncodeError(String::from("Unable to build Auth Token")))
    }
}

pub fn validate_jwt_for_scopes<'a>(
    req: &'a HttpRequest,
    scopes: &Vec<&str>,
) -> Result<GatewayUserClaims> {
    let jwt_config = req.app_data::<Data<JwtConfig>>().unwrap();
    let token = match req.headers().get("Authorization") {
        Some(auth_header) => {
            let auth_header = auth_header.to_str().unwrap_or("");
            let parts: Vec<&str> = auth_header.splitn(2, ' ').collect();
            if parts.len() != 2 || parts[0].to_lowercase() != "bearer" {
                ()
            }
            Some(parts[1])
        }
        None => None,
    }.ok_or(GatewayError::Unauthorized("Missing  'Bearer' token from 'Authorization' header".to_string()))?;
    let mut validation = Validation::new(jwt_config.algorithm);
    validation.set_audience(&scopes.iter().map(AsRef::as_ref).collect::<Vec<&str>>());
    validation.set_issuer(&[jwt_config.issuer.as_str()]);
    decode::<GatewayUserClaims>(token, &jwt_config.decoding_key, &validation)
        .and_then(|token_data| Ok(token_data.claims))
        .map_err(|e| GatewayError::TokenDecodeError(e.to_string()))
}
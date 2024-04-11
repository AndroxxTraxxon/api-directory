use actix_web::http::header;
use actix_web::{
    patch, post,
    web::{scope, to, Data, Json, Path, ServiceConfig},
    HttpRequest, HttpResponse,
};
use jsonwebtoken::{decode, encode, Header, Validation};
use serde_json::json;
use std::time::SystemTime;

use super::models::{
    GatewayLoginCredentials, GatewayUserClaims, JwtConfig, PasswordForm, RequestIdParams, UserForm,
};
use super::repo::UserAuthRepository;
use crate::database::Database;
use crate::errors::{unknown_resource_error, GatewayError, Result};

const GATEWAY_JWT_ISSUER: &str = "apigateway.local";

// Intermediate function to configure services
pub fn service_setup(cfg: &mut ServiceConfig) {
    cfg.service(
        scope("/auth/v1")
            .service(authenticate_user)
            .service(set_password)
            .service(request_password_reset)
            .service(reset_password)
            .default_service(to(unknown_resource_error)),
    );
}

#[post("/login")]
async fn authenticate_user(
    req: HttpRequest,
    repo: Data<Database>,
    credential_form: Json<GatewayLoginCredentials>,
) -> Result<String> {
    let credentials = credential_form.into_inner();
    let user =
        Database::authenticate_user(&repo, &credentials.username, &credentials.password).await?;
    let user_id = format!("{}", user.id.id);
    let now_ts = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let duration = 24 * 60 * 60;
    let claims = GatewayUserClaims {
        // Issuer (us/apigateway.local)
        iss: String::from(GATEWAY_JWT_ISSUER),
        sub: user.username.clone(),
        sub_id: user_id.clone(),
        aud: user.roles.iter().map(|role| format!("{}", role)).collect(),
        exp: now_ts + duration,
        iat: now_ts,
        nbf: now_ts,
    };
    let config: &Data<JwtConfig> = &req.app_data().unwrap();
    let token = encode(
        &Header::new(config.algorithm),
        &claims,
        &config.encoding_key,
    )
    .map_err(|e| GatewayError::TokenEncodeError(e.to_string()))?;

    Database::set_last_login(&repo, &user_id).await?;
    Ok(token)
}

#[patch("/set-password")]
async fn set_password(
    req: HttpRequest,
    repo: Data<Database>,
    password_form: Json<PasswordForm>,
) -> Result<HttpResponse> {
    let auth_claims = validate_jwt(&req, None)?;
    let user_id = validate_jwt(&req, None)?.sub_id;
    let request_form = password_form.into_inner();
    Database::authenticate_user(&repo, &auth_claims.sub, &request_form.old_password)
        .await
        .map_err(|e| match e {
            GatewayError::InvalidUsernameOrPassword(_) => {
                GatewayError::InvalidUsernameOrPassword("Old password does not match".to_string())
            }
            other => other,
        })?;
    Database::set_user_password(&repo, &user_id, &request_form.password).await?;
    Ok(HttpResponse::NoContent().finish())
}

#[post("/request-password-reset")]
async fn request_password_reset(
    repo: Data<Database>,
    user_form: Json<UserForm>,
) -> Result<HttpResponse> {
    let username = user_form.into_inner().username;
    let _ = Database::request_password_reset(&repo, &username).await;
    Ok(HttpResponse::Created().json(json!({"success": true, "message": format!("If a user exists with the username {}, they will receive a message to reset their password through the appropriate channel.", &username)})))
}

#[patch("/reset-password/{request_id}")]
async fn reset_password(
    repo: Data<Database>,
    credential_form: Json<GatewayLoginCredentials>,
    path_params: Path<RequestIdParams>,
) -> Result<HttpResponse> {
    let credentials = credential_form.into_inner();
    let request_id = path_params.into_inner().request_id;
    Database::set_user_password_with_reset_token(
        &repo,
        &request_id,
        &credentials.username,
        &credentials.password,
    )
    .await?;
    Ok(HttpResponse::NoContent().finish())
}

pub fn validate_jwt(req: &HttpRequest, scopes: Option<&Vec<&str>>) -> Result<GatewayUserClaims> {
    let jwt_config = req.app_data::<Data<JwtConfig>>().unwrap();
    let token = match req.headers().get(header::AUTHORIZATION) {
        Some(auth_header) => {
            let auth_header = auth_header.to_str().unwrap_or("");
            let parts: Vec<&str> = auth_header.splitn(2, ' ').collect();
            if parts.len() != 2 || parts[0].to_lowercase() != "bearer" {
                ()
            }
            Some(parts[1])
        }
        None => None,
    }
    .ok_or(GatewayError::Unauthorized(
        "Missing 'Bearer' token from 'Authorization' header".to_string(),
    ))?;

    let mut validation = Validation::new(jwt_config.algorithm);
    if let Some(audience) = scopes {
        validation.set_audience(audience);
    } else {
        validation.validate_aud = false;
    }
    validation.set_issuer(&[jwt_config.issuer.as_str()]);
    decode::<GatewayUserClaims>(token, &jwt_config.decoding_key, &validation)
        .and_then(|token_data| Ok(token_data.claims))
        .map_err(|e| GatewayError::TokenDecodeError(e.to_string()))
}

pub fn validate_jwt_prefix(
    req: &HttpRequest,
    scope_prefixes: &Vec<&str>,
) -> Result<GatewayUserClaims> {
    let jwt_config = req.app_data::<Data<JwtConfig>>().unwrap();
    let token = match req.headers().get(header::AUTHORIZATION) {
        Some(auth_header) => {
            let auth_header = auth_header.to_str().unwrap_or("");
            let parts: Vec<&str> = auth_header.splitn(2, ' ').collect();
            if parts.len() != 2 || parts[0].to_lowercase() != "bearer" {
                ()
            }
            Some(parts[1])
        }
        None => None,
    }
    .ok_or(GatewayError::Unauthorized(
        "Missing 'Bearer' token from 'Authorization' header".to_string(),
    ))?;

    let mut validation = Validation::new(jwt_config.algorithm);
    validation.validate_aud = false;
    let claims = decode::<GatewayUserClaims>(token, &jwt_config.decoding_key, &validation)
        .and_then(|token_data| Ok(token_data.claims))
        .map_err(|e| GatewayError::TokenDecodeError(e.to_string()))?;

    if !claims
        .aud
        .iter()
        .any(|a| scope_prefixes.iter().any(|p| a.starts_with(p)))
    {
        return Err(GatewayError::TokenDecodeError(
            "User does not have the requisite role".into(),
        ));
    }
    Ok(claims)
}

use actix_web::{http::StatusCode, HttpResponse, Responder, ResponseError};

use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GatewayError {
    /**
     * API Service Errors
     */

    #[error("{0} Record not found: {1}")]
    NotFound(String, String),

    #[error("Missing data: {0}")]
    MissingData(String),

    // Uncomment and adapt these as needed
    // #[error("Registration failure: {0}")]
    // RegistrationFailure(String),

    // #[error("Duplicate service: {0}")]
    // DuplicateService(String),
    #[error("Not authorized: {0}")]
    Unauthorized(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    // #[error("Not implemented: {0}")]
    // NotImplemented(String),

    /**
     * Auth Errors
     */

    #[error("Invalid Username or Password: {0}")]
    InvalidUsernameOrPassword(String),
    // #[error("There was an error authenticating: {0}")]
    // UnableToAuthenticate(String),
    #[error("Unable to decode auth token: {0}")]
    TokenDecodeError(String),
    #[error("Unable to encode auth token: {0}")]
    TokenEncodeError(String),
    // #[error("Authentication Configuration error: {0}")]
    // ConfigError(String)
    // #[error("Missing Authorization Scope: {0}")]
    // AccessDenied(String),

    #[error("System Error: {0}")]
    SystemError(String),

    #[error("Bad Request: {0}")]
    BadRequest(String),
}

impl ResponseError for GatewayError {
    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        HttpResponse::build(self.status_code())
            .json(json!({"success": false, "error": self.to_string()}))
    }

    fn status_code(&self) -> StatusCode {
        match self {
            GatewayError::NotFound(_, _) => StatusCode::NOT_FOUND,
            GatewayError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            GatewayError::MissingData(_) => StatusCode::BAD_REQUEST,
            GatewayError::BadRequest(_) => StatusCode::BAD_REQUEST,
            GatewayError::InvalidUsernameOrPassword(_) => StatusCode::UNAUTHORIZED,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

pub async fn unknown_resource_error() -> impl Responder {
    HttpResponse::BadRequest().json(json!({"success": false, "error": "Unknown Resource URL"}))
}

pub type Result<T> = core::result::Result<T, GatewayError>;

impl From<surrealdb::Error> for GatewayError {
    fn from(value: surrealdb::Error) -> Self {
        GatewayError::DatabaseError(value.to_string())
    }
}

use actix_web::{
    http::StatusCode,
    HttpResponse, ResponseError,
};

use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Invalid Username or Password {0}")]
    InvalidUsernameOrPassword(String),
    // #[error("There was an error authenticating: {0}")]
    // UnableToAuthenticate(String),
    #[error("Unable to decode auth token: {0}")]
    TokenDecodeError(String),
    #[error("Unable to encode auth token: {0}")]
    TokenEncodeError(String),
    #[error("Database Error: {0}")]
    DatabaseError(String),
    #[error("Method not implemented: {0}")]
    NotImplemented(String),
    // #[error("Authentication Configuration error: {0}")]
    // ConfigError(String)
}


impl ResponseError for AuthError {
    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        HttpResponse::build(self.status_code())
            .json(json!({"success": false, "error": self.to_string()}))
    }

    fn status_code(&self) -> StatusCode {
        match self {
            AuthError::InvalidUsernameOrPassword(_) => StatusCode::FORBIDDEN,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
use actix_web::{
    http::StatusCode,
    HttpResponse, ResponseError
};

use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum UserError {
    #[error("User Not Found: {0}")]
    UserNotFound(String),
    // #[error("User Registration Failure: {0}")]
    // UserRegistrationFailure(String),
    // #[error("Authentication Failure: {0}")]
    // AuthenticationFailure(String),
    // #[error("Validation Error: {0}")]
    // ValidationError(String),
    #[error("Not Implemented: {0}")]
    NotImplemented(String),
    #[error("Database Error: {0}")]
    DatabaseError(String)
}

impl ResponseError for UserError {
    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        HttpResponse::build(self.status_code())
            .json(json!({"success": false, "error": self.to_string()}))
    }

    fn status_code(&self) -> StatusCode {
        match self {
            UserError::UserNotFound(_) => StatusCode::NOT_FOUND,
            // UserError::UserRegistrationFailure(_) => StatusCode::BAD_REQUEST,
            // UserError::AuthenticationFailure(_) => StatusCode::BAD_REQUEST,
            // UserError::ValidationError(_) => StatusCode::BAD_REQUEST,
            UserError::NotImplemented(_) => StatusCode::INTERNAL_SERVER_ERROR,
            UserError::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
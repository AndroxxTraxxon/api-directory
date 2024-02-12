use actix_web::{
    http::StatusCode,
    HttpResponse, ResponseError
};

use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiServiceError {
    #[error("Service not found: {0}")]
    ServiceNotFound(String),

    #[error("Missing data: {0}")]
    MissingData(String),

    // Uncomment and adapt these as needed
    // #[error("Registration failure: {0}")]
    // RegistrationFailure(String),
    
    // #[error("Duplicate service: {0}")]
    // DuplicateService(String),
    
    // #[error("Not authorized: {0}")]
    // NotAuthorized(String),
    
    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Not implemented: {0}")]
    NotImplemented(String),
}

impl ResponseError for ApiServiceError {
    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        HttpResponse::build(self.status_code())
            .json(json!({"success": false, "error": self.to_string()}))
    }

    fn status_code(&self) -> StatusCode {
        match self {
            ApiServiceError::ServiceNotFound(_) => StatusCode::NOT_FOUND,
            // ApiServiceError::RegistrationFailure(_) => StatusCode::INTERNAL_SERVER_ERROR,
            // ApiServiceError::DuplicateService(_) => StatusCode::BAD_REQUEST,
            // ApiServiceError::NotAuthorized(_) => StatusCode::UNAUTHORIZED,
            ApiServiceError::MissingData(_) => StatusCode::BAD_REQUEST,
            ApiServiceError::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiServiceError::NotImplemented(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
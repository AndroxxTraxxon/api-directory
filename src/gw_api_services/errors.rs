use actix_web::{
    http::{header::ContentType, StatusCode },
    HttpResponse, ResponseError
};

use derive_more::Display;

#[derive(Debug, Display)]
pub enum ApiServiceError {
    ServiceNotFound,
    MissingData,
    RegistrationFailure,
    DuplicateService,
    NotAuthorized
}

impl ResponseError for ApiServiceError {
    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::json())
            .body(self.to_string())
    }

    fn status_code(&self) -> StatusCode {
        match self {
            ApiServiceError::ServiceNotFound => StatusCode::NOT_FOUND,
            ApiServiceError::RegistrationFailure => StatusCode::INTERNAL_SERVER_ERROR,
            ApiServiceError::DuplicateService => StatusCode::BAD_REQUEST,
            ApiServiceError::NotAuthorized => StatusCode::UNAUTHORIZED,
            ApiServiceError::MissingData => StatusCode::BAD_REQUEST
        }
    }
}
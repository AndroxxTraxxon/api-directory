use super::{
    errors::ApiServiceError,
    models::{ApiService, PartialApiServiceUpdate},
    repo::ApiServiceRepository,
};
use crate::database::Database;
use actix_web::{
    delete, get, patch, post,
    web::{scope, Data, Json, Path, ServiceConfig},
    HttpResponse,
};
use serde::Deserialize;

// Intermediate function to configure services
pub fn web_setup(cfg: &mut ServiceConfig) {
    cfg.service(
        scope("/cfg/v1")
            .service(list_services)
            .service(add_service)
            .service(patch_service)
            // .service(http::update_service)
            .service(get_service_by_name_and_version)
            .service(delete_service),
    );
}

#[get("/api_services")]
async fn list_services(repo: Data<Database>) -> Result<Json<Vec<ApiService>>, ApiServiceError> {
    let api_services = Database::get_all_services(&repo).await?;
    Ok(Json(api_services))
}

#[derive(Deserialize)]
struct ApiServiceNamedVersionPath {
    pub api_name: String,
    pub version: String,
}

#[get("/api_services/{api_name}/{version}")]
async fn get_service_by_name_and_version(
    path_params: Path<ApiServiceNamedVersionPath>,
    repo: Data<Database>,
) -> Result<Json<ApiService>, ApiServiceError> {
    let parsed_path = path_params.into_inner();
    let api_name = parsed_path.api_name;
    let version = parsed_path.version;
    let found_service = Database::get_service_by_name_and_version(&repo, &api_name, &version).await?;
    Ok(Json(found_service))
    
}

#[post("/api_services")]
async fn add_service(service: Json<ApiService>, repo: Data<Database>) -> Result<Json<ApiService>, ApiServiceError> {
    let created_service = Database::add_service(&repo, &service.into_inner()).await?;
    Ok(Json(created_service))
}

#[derive(Deserialize)]
struct ApiServiceIdPath {
    pub service_id: String,
}

#[patch("/api_services/{service_id}")]
async fn patch_service(
    path_params: Path<ApiServiceIdPath>,
    service: Json<PartialApiServiceUpdate>,
    repo: Data<Database>,
) -> Result<Json<ApiService>, ApiServiceError> {
    let service_id = path_params.into_inner().service_id;
    let patched_service = Database::patch_service(&repo, &service_id, &service.into_inner()).await?;
    Ok(Json(patched_service))
}

#[delete("/api_services/{service_id}")]
async fn delete_service(
    path_params: Path<ApiServiceIdPath>,
    repo: Data<Database>,
) -> Result<HttpResponse, ApiServiceError>{
    let service_id = path_params.into_inner().service_id;
    Database::delete_service(&repo, service_id.as_str()).await?;
    Ok(HttpResponse::NoContent().finish())
}

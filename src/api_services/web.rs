use actix_web::{
    delete, get, patch, post,
    web::{scope, to, Data, Json, Path, ServiceConfig},
    HttpRequest, HttpResponse,
};
use serde::Deserialize;

use crate::auth::web::validate_jwt;
use crate::database::Database;
use crate::errors::{Result, unknown_resource_error};

use super::{
    models::{ApiService, PartialApiServiceUpdate},
    repo::ApiServiceRepository,
};

// Intermediate function to configure services
pub fn service_setup(cfg: &mut ServiceConfig) {
    cfg.service(
        scope("/cfg/v1/api_services")
            .service(list_services)
            .service(add_service)
            .service(patch_service)
            // .service(http::update_service)
            .service(get_service_by_name_and_version)
            .service(delete_service)
            .default_service(to(unknown_resource_error))
    );
}

#[get("/")]
async fn list_services(req: HttpRequest, repo: Data<Database>) -> Result<Json<Vec<ApiService>>> {
    validate_jwt(&req, Some(&vec!["admin", "services-readonly"]))?;
    let api_services = Database::list_services(&repo).await?;
    Ok(Json(api_services))
}

#[derive(Deserialize)]
struct ApiServiceNamedVersionPath {
    pub api_name: String,
    pub version: String,
}

#[get("/{api_name}/{version}")]
async fn get_service_by_name_and_version(
    req: HttpRequest,
    path_params: Path<ApiServiceNamedVersionPath>,
    repo: Data<Database>,
) -> Result<Json<ApiService>> {
    validate_jwt(&req, Some(&vec!["admin", "services-readonly"]))?;
    let parsed_path = path_params.into_inner();
    let api_name = parsed_path.api_name;
    let version = parsed_path.version;
    let found_service =
        Database::get_service_by_name_and_version(&repo, &api_name, &version).await?;
    Ok(Json(found_service))
}

#[post("/")]
async fn add_service(
    req: HttpRequest,
    service: Json<ApiService>,
    repo: Data<Database>,
) -> Result<Json<ApiService>> {
    validate_jwt(&req, Some(&vec!["admin"]))?;
    let created_service = Database::add_service(&repo, &service.into_inner()).await?;
    Ok(Json(created_service))
}

#[derive(Deserialize)]
struct ApiServiceIdPath {
    pub service_id: String,
}

#[patch("/{service_id}")]
async fn patch_service(
    req: HttpRequest,
    path_params: Path<ApiServiceIdPath>,
    service: Json<PartialApiServiceUpdate>,
    repo: Data<Database>,
) -> Result<Json<ApiService>> {
    validate_jwt(&req, Some(&vec!["admin"]))?;
    let service_id = path_params.into_inner().service_id;
    let patched_service =
        Database::patch_service(&repo, &service_id, &service.into_inner()).await?;
    Ok(Json(patched_service))
}

#[delete("/{service_id}")]
async fn delete_service(
    req: HttpRequest,
    path_params: Path<ApiServiceIdPath>,
    repo: Data<Database>,
) -> Result<HttpResponse> {
    validate_jwt(&req, Some(&vec!["admin"]))?;
    let service_id = path_params.into_inner().service_id;
    Database::delete_service(&repo, service_id.as_str()).await?;
    Ok(HttpResponse::NoContent().finish())
}

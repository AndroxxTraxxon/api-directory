use super::repo::ApiServiceRepository;
use super::models::{ApiService, PartialApiServiceUpdate};
use crate::gw_database::Database;
use actix_web::{
    delete, get, patch, post,
    web::{Data, Json, Path, ServiceConfig, scope},
    HttpResponse, Responder,
};
use serde::Deserialize;
use serde_json::json;

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
async fn list_services(repo: Data<Database>) -> impl Responder {
    let services = Database::get_all_services(&repo).await;
    log::warn!("Fetching list of API services...");
    match services {
        Some(found_services) => HttpResponse::Ok().json(found_services), // Automatically serializes to JSON
        None => {
            HttpResponse::InternalServerError().json(json!({"error": "Error fetching records"}))
        } // Provides a JSON error message
    }
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
) -> impl Responder {
    let parsed_path = path_params.into_inner();
    let api_name = parsed_path.api_name;
    let version = parsed_path.version;
    let found_service = Database::get_service_by_name_and_version(&repo, api_name.clone(), version.clone()).await;
    match found_service {
        Some(api_service) => HttpResponse::Ok().json(api_service),
        None => HttpResponse::NotFound().json(json!({
            "error": format!("No active API Service is registered with API name '{}' and version '{}'", api_name, version)
        }))
    }
}

#[post("/api_services")]
async fn add_service(service: Json<ApiService>, repo: Data<Database>) -> impl Responder {
    let created_service = Database::add_service(&repo, service.into_inner()).await;
    match created_service {
        Some(new_service) => HttpResponse::Ok().json(new_service),
        None => {
            HttpResponse::InternalServerError().json(json!({"error": "Error registering service"}))
        }
    }
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
) -> impl Responder {
    let service_id = path_params.into_inner().service_id;
    let patch_result = Database::patch_service(&repo, service_id, service.into_inner()).await;
    match patch_result {
        Some(patched_service) => HttpResponse::Ok().body(format!("{:?}", patched_service)),
        None => HttpResponse::Ok().body("Error Updating record"),
    }
}

#[delete("/api_services/{service_id}")]
async fn delete_service(
    path_params: Path<ApiServiceIdPath>,
    repo: Data<Database>,
) -> impl Responder {
    let service_id = path_params.into_inner().service_id;
    let db_result = Database::delete_service(&repo, service_id.as_str()).await;
    match db_result{
        Some(_) => HttpResponse::NoContent().json(json!({"success": true})),
        None => HttpResponse::NotFound().json(json!({"error": "Endpoint not found.", "success": false}))
    }
}

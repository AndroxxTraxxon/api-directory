use actix_web::{delete, get, post, put, web, HttpResponse, Responder};
use serde::Deserialize;

use crate::services::types::{ApiConfig, AppState}; // Assuming AppState is moved to services module

#[derive(Deserialize)]
struct NamedServicePath {
    pub name: String,
}

#[get("/services")]
async fn list_services(data: web::Data<AppState>) -> impl Responder {
    let services = data.apis.lock().unwrap();
    HttpResponse::Ok().json(&*services)
}

#[post("/services")]
async fn add_service(service: web::Json<ApiConfig>, data: web::Data<AppState>) -> impl Responder {
    let mut services = data.apis.lock().unwrap();
    services.push(service.into_inner());
    HttpResponse::Ok().body("service added")
}

#[put("/services/{name}")]
async fn update_service(
    path_params: web::Path<NamedServicePath>,
    service: web::Json<ApiConfig>,
    data: web::Data<AppState>,
) -> impl Responder {
    let mut services = data.apis.lock().unwrap();
    match services.iter_mut().find(|x| x.name == path_params.name) {
        Some(existing_service) => {
            *existing_service = service.into_inner();
            HttpResponse::Ok().body("Service updated")
        }
        None => HttpResponse::NotFound().body("Service not found"),
    }
}

#[delete("/services/{name}")]
async fn delete_service(
    path_params: web::Path<NamedServicePath>,
    data: web::Data<AppState>,
) -> impl Responder {
    let mut services = data.apis.lock().unwrap();
    if let Some(index) = services.iter().position(|x| x.name == path_params.name) {
        services.remove(index);
        HttpResponse::Ok().body("Service removed")
    } else {
        HttpResponse::NotFound().body("Service not found")
    }
}

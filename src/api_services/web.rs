use crate::api_services::db::ApiServiceRepository;
use crate::api_services::models::ApiService;
use crate::database::Database;
use actix_web::{get, post, web, HttpResponse, Responder};



#[get("/services")]
async fn list_services(db: web::Data<Database>) -> impl Responder {
    let services = Database::get_all_services(&db).await;
    match services {
        Some(found_services) => HttpResponse::Ok().body(format!("{:?}", found_services)),
        None => HttpResponse::Ok().body("Error fetching records"),
    }
}

#[post("/services")]
async fn add_service(service: web::Json<ApiService>, db: web::Data<Database>) -> impl Responder {
    let created_service = Database::add_service(&db, service.into_inner()).await;
    match created_service {
        Some(new_service) => HttpResponse::Ok().body(format!("{:?}", new_service)),
        None => HttpResponse::Ok().body("Error Creating record"),
    }
}

// use serde::Deserialize;

// #[derive(Deserialize)]
// struct NamedServicePath {
//     pub name: String,
// }

// #[put("/services/{name}")]
// async fn update_service(
//     path_params: web::Path<NamedServicePath>,
//     service: web::Json<ApiService>,
//     data: web::Data<Database>,
// ) -> impl Responder {
//     let mut services = data.apis.lock().unwrap();
//     match services.iter_mut().find(|x| x.name == path_params.name) {
//         Some(existing_service) => {
//             *existing_service = service.into_inner();
//             HttpResponse::Ok().body("Service updated")
//         }
//         None => HttpResponse::NotFound().body("Service not found"),
//     }
// }

// #[delete("/services/{name}")]
// async fn delete_service(
//     path_params: web::Path<NamedServicePath>,
//     data: web::Data<Database>,
// ) -> impl Responder {
//     let mut services = data.apis.lock().unwrap();
//     if let Some(index) = services.iter().position(|x| x.name == path_params.name) {
//         services.remove(index);
//         HttpResponse::Ok().body("Service removed")
//     } else {
//         HttpResponse::NotFound().body("Service not found")
//     }
// }

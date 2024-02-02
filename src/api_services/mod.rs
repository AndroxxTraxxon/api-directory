use actix_web;

pub mod rest; // Importing the rest module
pub mod models;
pub mod db;

// Intermediate function to configure services
pub fn web_setup(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(
        actix_web::web::scope("/cfg/v1")
            .service(rest::list_services)
            .service(rest::add_service)
            .service(rest::patch_service)
            .service(rest::get_service_by_name_and_version)
            // .service(http::update_service)
            .service(rest::delete_service),
    );
}
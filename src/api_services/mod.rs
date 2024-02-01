use actix_web;

pub mod web; // Importing the rest module
pub mod models;
pub mod db;

// Intermediate function to configure services
pub fn web_setup(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(
        actix_web::web::scope("/services")
            .service(web::list_services)
            .service(web::add_service)
            // .service(http::update_service)
            // .service(http::delete_service),
    );
}
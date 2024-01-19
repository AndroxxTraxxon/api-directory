use actix_web::web;

pub mod rest_interface; // Importing the rest module
pub mod types;




// Intermediate function to configure services
pub fn config_services(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/services")
            .service(rest_interface::list_services)
            .service(rest_interface::add_service)
            .service(rest_interface::update_service)
            .service(rest_interface::delete_service),
    );
}

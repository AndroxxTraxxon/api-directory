use std::sync::Mutex;

use actix_web::{App, HttpServer, web};
use actix_web::middleware::Logger;
mod services; // Import the services module
mod proxy;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let app_data = web::Data::new(services::types::AppState {
        apis: Mutex::new(Vec::new()),
    });

    println!("Starting HTTP server at http://127.0.0.1:8080");
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::new("%a \"%r\" %s %b \"%{Referer}i\" \"%{User-Agent}i\" %T"))
            .app_data(service_registry.clone())
            // ... existing services ...
            .default_service(
                // Register `forward` as the default service
                web::route().to(forward)
            )
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

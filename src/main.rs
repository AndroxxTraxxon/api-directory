use actix_web::{middleware, web, App, HttpServer};
mod proxy;
mod services; // Import the services module

use env_logger;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

    let service_registry = web::Data::new(std::sync::Arc::new(services::types::ServiceRegistry {
        services: {
            let mut map = std::collections::HashMap::new();
            map.insert("example".to_string(), "http://example.com".to_string());
            // Add your services here
            map
        },
    }));

    println!("Starting HTTP server at http://127.0.0.1:8080");
    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::new(
                "%a \"%r\" %s %b \"%{Referer}i\" \"%{User-Agent}i\" %T",
            ))
            .app_data(service_registry.clone())
            .configure(services::config_services)
            .default_service(
                // Register `forward` as the default service
                web::route().to(proxy::forward),
            )
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

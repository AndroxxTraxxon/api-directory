use actix_web::{middleware, web, App, HttpServer};
use env_logger;

mod proxy;
mod database;
mod api_services;


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

    let db = database::Database::init(
        "file://~/temp.speedb",
        "api_directory",
        "services",
    )
    .await
    .expect("Error connecting to database");

    let db_data = web::Data::new(db);

    println!("Starting HTTP server at http://127.0.0.1:8080");
    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::new(
                "%a \"%r\" %s %b \"%{Referer}i\" \"%{User-Agent}i\" %T",
            ))
            .app_data(db_data.clone())
            .configure(api_services::web_setup)
            .default_service(
                // Register `forward` as the default service
                web::route().to(proxy::forward),
            )
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

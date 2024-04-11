use actix_files;
use actix_web::{middleware, web, App, HttpResponse, HttpServer};
use env_logger;
use serde_json::json;

mod api_services;
mod auth;
mod database;
mod errors;
mod forwarder;
mod secconf;
mod users;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
    let socket_addr = "127.0.2.1:443";
    let tls_config = secconf::load_tls_config()?;
    let jwt_config = web::Data::new(secconf::load_jwt_config()?);
    let db = database::Database::init("temp.speedb", "api_directory", "services")
        .await
        .expect("Error connecting to database");

    api_services::repo::setup_service_table_events(&db).await?;
    users::repo::setup_user_table(&db).await?;
    auth::repo::setup_reset_request_table(&db).await?;

    let db_data = web::Data::new(db);

    log::info!("Starting HTTP server at https://{} ", socket_addr);
    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::new(
                "%a \"%r\" %s %b \"%{Referer}i\" \"%{User-Agent}i\" %T",
            ))
            .wrap(secconf::load_cors_config())
            .app_data(db_data.clone())
            .app_data(jwt_config.clone())
            .configure(api_services::web::service_setup)
            .configure(auth::web::service_setup)
            .configure(users::web::service_setup)
            .service(web::scope("/cfg").default_service(web::route().to(not_found)))
            .service(actix_files::Files::new("/app", "./www").index_file("index.html"))
            .service(web::redirect("/", "/app"))
            .default_service(
                // Register `forward` as the default service
                web::route().to(forwarder::forward),
            )
    })
    .bind_rustls_0_22(socket_addr, tls_config)?
    .run()
    .await
}

async fn not_found() -> impl actix_web::Responder {
    HttpResponse::NotFound().json(json!({"success": false, "error": "Unknown Config Service"}))
}

use actix_web::{middleware, web, App, HttpServer};
use env_logger;

mod forwarder;
mod database;
mod api_services;
mod users;
mod tlsconf;


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
    let socket_addr = "127.0.2.1:443";
    let tls_config = tlsconf::load_tls_config();
    let db = database::Database::init(
        "temp.speedb",
        "api_directory",
        "services",
    )
    .await
    .expect("Error connecting to database");
    users::repo::setup_user_table_events(&db)
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string().as_str()))?;

    let db_data = web::Data::new(db);
    
    log::info!("Starting HTTP server at https://{} ", socket_addr);
    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::new(
                "%a \"%r\" %s %b \"%{Referer}i\" \"%{User-Agent}i\" %T",
            ))
            .app_data(db_data.clone())
            .configure(api_services::rest::web_setup)
            .default_service(
                // Register `forward` as the default service
                web::route().to(forwarder::forward),
            )
    })
    .bind_rustls_0_22(socket_addr, tls_config)?
    .run()
    .await
}
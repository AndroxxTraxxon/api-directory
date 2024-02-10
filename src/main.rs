use std::{fs::File, io::BufReader};

use actix_web::{middleware, web, App, HttpServer};
use env_logger;
use rustls::pki_types::PrivateKeyDer;
use rustls_pemfile::{certs, pkcs8_private_keys};

mod gw_proxy;
mod gw_database;
mod gw_api_services;
mod gw_users;


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
    let socket_addr = "127.0.2.1:8443";
    let tls_config = load_tls_config();
    let db = gw_database::Database::init(
        "temp.speedb",
        "api_directory",
        "services",
    )
    .await
    .expect("Error connecting to database");

    let db_data = web::Data::new(db);
    
    log::info!("Starting HTTP server at https://{} ", socket_addr);
    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::new(
                "%a \"%r\" %s %b \"%{Referer}i\" \"%{User-Agent}i\" %T",
            ))
            .app_data(db_data.clone())
            .configure(gw_api_services::rest::web_setup)
            .default_service(
                // Register `forward` as the default service
                web::route().to(gw_proxy::forward),
            )
    })
    .bind_rustls_0_22(socket_addr, tls_config)?
    .run()
    .await
}

fn load_tls_config() -> rustls::ServerConfig{
    let config = rustls::ServerConfig::builder()
        .with_no_client_auth();

    let certificate_file = &mut BufReader::new(
        File::open(".ssl.dev/snakeoil.pem").unwrap()
    );
    let key_file = &mut BufReader::new(
        File::open(".ssl.dev/snakeoil.key").unwrap()
    );

    let cert_chain = certs(certificate_file)
        .filter_map(Result::ok)
        .collect();

    let mut keys: Vec<_> = pkcs8_private_keys(key_file)
        .filter_map(Result::ok)
        .map(|pkcs8_key| PrivateKeyDer::Pkcs8(pkcs8_key))
        .collect();

    if keys.is_empty() {
        eprintln!("Could not locate PKCS 8 private keys.");
        std::process::exit(1);
    }

    config.with_single_cert(cert_chain, keys.remove(0)).unwrap()

}

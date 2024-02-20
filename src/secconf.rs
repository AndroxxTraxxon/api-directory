use std::{fs::File, io::BufReader};

use rustls::pki_types::PrivateKeyDer;
use rustls_pemfile::{certs, pkcs8_private_keys};

use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey};

use crate::auth::models::JwtConfig;

pub fn load_tls_config() -> std::io::Result<rustls::ServerConfig> {
    let config = rustls::ServerConfig::builder().with_no_client_auth();

    let certificate_file = &mut BufReader::new(File::open(".ssl.dev/snakeoil.pem")?);
    let key_file = &mut BufReader::new(File::open(".ssl.dev/snakeoil.key")?);

    let cert_chain = certs(certificate_file).filter_map(Result::ok).collect();

    let mut keys: Vec<_> = pkcs8_private_keys(key_file)
        .filter_map(Result::ok)
        .map(|pkcs8_key| PrivateKeyDer::Pkcs8(pkcs8_key))
        .collect();

    if keys.is_empty() {
        eprintln!("Could not locate PKCS 8 private keys.");
        std::process::exit(1);
    }

    Ok(config.with_single_cert(cert_chain, keys.remove(0)).unwrap())
}

pub fn load_jwt_config() -> std::io::Result<JwtConfig> {
    Ok(JwtConfig {
        algorithm: Algorithm::RS512,
        decoding_key: DecodingKey::from_rsa_pem(&std::fs::read(".ssl.dev/snakeoil.pem")?)
            .map_err(|e| std::io::Error::other(e))?,
        encoding_key: EncodingKey::from_rsa_pem(&std::fs::read(".ssl.dev/snakeoil.key")?)
            .map_err(|e| std::io::Error::other(e))?,
        issuer: String::from("apigateway.local"),
    })
}

pub fn load_cors_config() -> actix_cors::Cors {
    // CORS policy will need to be very permissive since this is an API gateway.
    actix_cors::Cors::permissive()
}

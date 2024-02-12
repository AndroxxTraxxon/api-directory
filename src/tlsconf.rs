use std::{fs::File, io::BufReader};
use rustls::pki_types::PrivateKeyDer;
use rustls_pemfile::{certs, pkcs8_private_keys};

pub fn load_tls_config() -> rustls::ServerConfig{
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

use crate::{
    api_services::{models::DbApiRole, repo::ApiServiceRepository},
    auth::web::validate_jwt,
    database::{Database, NAMESPACE_MEMBER_ROLE, ROLE_NAMESPACE_DELIMITER},
    errors::GatewayError,
};
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use futures_util::stream::TryStreamExt;
use reqwest::Client;

const EXCLUDE_HEADERS: &[&str] = &["host"];

pub async fn forward(
    req: HttpRequest,
    payload: web::Payload,
    db: web::Data<Database>,
) -> impl Responder {
    let segments: Vec<&str> = req.path().splitn(4, '/').collect();

    // Just validate that the token is valid and not expired. aud/roles will be checked later.
    let claims =
        validate_jwt(&req, None).map_err(|e| actix_web::error::ErrorForbidden(e.to_string()))?;

    if segments.len() != 4 {
        return Ok(HttpResponse::BadRequest().finish());
    }
    let api_name = String::from(segments[1]);
    let version = String::from(segments[2]);
    let endpoint = String::from(segments[3]);

    log::debug!(
        "Forwarding request to service [{}] version [{}] at endpoint {}",
        api_name,
        version,
        endpoint
    );

    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|err| GatewayError::SystemError(err.to_string()))?;

    if let Ok(service) = Database::get_service_with_roles(&db, &api_name, &version).await {
        // Construct the full URL
        if !check_aud_authorized(&service.roles, &claims.aud) {
            return Err(actix_web::error::ErrorForbidden(
                "User roles do not authorize access to this service",
            ));
        }
        log::debug!("Configured Forward URL: {}", service.forward_url);
        let forward_url = format!("{}/{}", service.forward_url, endpoint);

        log::info!(
            "{} -> {}[{}] as {} \"{} {}\"",
            req.peer_addr().unwrap().ip().to_string(),
            &api_name,
            &version,
            &claims.sub_id,
            &req.method(),
            &forward_url,
        );

        // Initialize the client request
        let mut client_req = client.request(req.method().clone(), &forward_url);

        // Copy the headers
        for (key, value) in req
            .headers()
            .iter()
            .filter(|(key, _)| !EXCLUDE_HEADERS.contains(&key.as_str()))
        {
            log::debug!(
                "Passing header: {}: {:?}",
                key,
                value.clone().to_str().unwrap()
            );
            client_req = client_req.header(key.clone(), value.clone());
        }

        // Set additional headers for forwarding
        client_req = client_req
            .header("X-Real-IP", req.peer_addr().unwrap().ip().to_string())
            .header(
                "X-Forwarded-For",
                req.connection_info().realip_remote_addr().unwrap_or(""),
            )
            .header("X-Forwarded-Proto", req.connection_info().scheme())
            .header("X-Forwarded-Host", req.connection_info().host());

        // Stream the request body
        let body_stream = payload
            .try_fold(web::BytesMut::new(), |mut body, chunk| {
                body.extend_from_slice(&chunk);
                async move { Ok(body) }
            })
            .await
            .map_err(actix_web::error::Error::from)?;

        client_req = client_req.body(body_stream.freeze().to_vec());

        // Send the request
        let response = client_req.send().await.map_err(|e| {
            eprintln!("Error forwarding request: {}", e);
            actix_web::error::ErrorInternalServerError("Error forwarding request")
        })?;

        // Convert the response into an Actix HttpResponse and return it
        let mut builder = HttpResponse::build(response.status());
        for (key, value) in response.headers().iter() {
            builder.insert_header((key.clone(), value.clone()));
        }

        let body = response.bytes().await.map_err(|e| {
            eprintln!("Error reading response body: {}", e);
            actix_web::error::ErrorInternalServerError("Error reading response body")
        })?;
        Ok(builder.body(body))
    } else {
        Ok(HttpResponse::NotFound().finish())
    }
}

fn check_aud_authorized(service_roles: &Vec<DbApiRole>, claims_aud: &Vec<String>) -> bool {
    // Convert service roles to a HashSet for efficient lookup
    if service_roles
        .iter()
        .any(|role| claims_aud.contains(&format!("{}", role)))
    {
        log::debug!("Found exact role match");
        return true;
    }

    let namespaces: Vec<String> = service_roles
        .iter()
        .filter(|r| r.name.eq(NAMESPACE_MEMBER_ROLE))
        .map(|r| r.namespace.clone())
        .collect();
    dbg!(&namespaces);
    dbg!(claims_aud);

    if !namespaces.is_empty() {
        if claims_aud.iter().any(|aud| {
            namespaces.iter().any(|ns| {
                aud.starts_with(&format!("{}{}", ns, ROLE_NAMESPACE_DELIMITER.to_string()))
            })
        }) {
            log::debug!("Found namespace match");
            return true;
        }
    }
    log::debug!("No matching aud found");
    false // No matching scopes found
}

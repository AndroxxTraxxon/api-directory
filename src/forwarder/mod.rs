use std::collections::HashSet;

use crate::{
    api_services::repo::ApiServiceRepository,
    database::Database,
    auth::web::validate_jwt,
};
use actix_web::{web, HttpRequest, HttpResponse, Result};
use futures_util::stream::TryStreamExt;
use reqwest::Client;

const EXCLUDE_HEADERS: &[&str] = &["host"];

pub async fn forward(
    req: HttpRequest,
    payload: web::Payload,
    db: web::Data<Database>,
) -> Result<HttpResponse> {

    let segments: Vec<&str> = req.path().splitn(4, '/').collect();

    // Just validate that the token is valid and not expired. scopes will be checked later.
    let claims = validate_jwt(&req, None)
        .map_err(|e| actix_web::error::ErrorForbidden(e.to_string()))?;

    if segments.len() != 4 {
        return Ok(HttpResponse::BadRequest().finish());
    }
    let api_name = String::from(segments[1]);
    let version = String::from(segments[2]);
    let endpoint = String::from(segments[3]);

    log::debug!("Forwarding request to service [{}] version [{}] at endpoint {}", api_name, version, endpoint);

    let client = Client::new();

    if let Ok(service) = Database::get_service_by_name_and_version(&db, &api_name, &version).await {
        // Construct the full URL
        let service_scopes: Vec<&str> = service.gateway_scopes.iter().map(AsRef::as_ref).collect();
        let authorized_scopes: Vec<String> = claims.aud;
        if !check_scope_intersection(service_scopes, authorized_scopes) {
            return Err(actix_web::error::ErrorForbidden("User is not authorized for service scopes"))
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
        for (key, value) in req.headers().iter().filter(|(key, _)| !EXCLUDE_HEADERS.contains(&key.as_str()))  {
            log::debug!("Passing header: {}: {:?}", key, value.clone().to_str().unwrap());
            client_req = client_req.header(key.clone(), value.clone());
        }

        // Set additional headers for forwarding
        client_req = client_req.header("X-Real-IP", req.peer_addr().unwrap().ip().to_string())
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

fn check_scope_intersection(service_scopes: Vec<&str>, authorized_scopes: Vec<String>) -> bool {
    // Convert authorized_scopes to a HashSet for efficient lookup
    let authorized_set: HashSet<&String> = authorized_scopes.iter().collect();

    // Iterate over service_scopes and check for any intersection
    for scope in service_scopes {
        if authorized_set.contains(&scope.to_string()) {
            return true; // Found a matching scope, can return early
        }
    }

    false // No matching scopes found
}


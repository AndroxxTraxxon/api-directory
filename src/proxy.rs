use crate::services::types::ServiceRegistry;
use actix_web::{web, Error, HttpRequest, HttpResponse, Result};
use futures_util::stream::TryStreamExt;
use reqwest::Client;
use std::sync::Arc;
use lazy_static::lazy_static;

lazy_static! {
    static ref EXCLUDE_HEADERS: std::collections::HashSet<&'static str> = {
        let mut set = std::collections::HashSet::new();
        set.insert("host");
        // Add more headers to exclude as needed
        set
    };
}

pub async fn forward(
    req: HttpRequest,
    payload: web::Payload,
    data: web::Data<Arc<ServiceRegistry>>,
) -> Result<HttpResponse> {

    let path = req.path().to_string();
    let segments: Vec<&str> = path.splitn(3, '/').collect();

    if segments.len() != 3 {
        return Ok(HttpResponse::BadRequest().finish());
    }
    let service_name = segments[1].to_string();
    let endpoint = segments[2].to_string();

    log::debug!("Forwarding request to service {} at endpoint {}", service_name, endpoint);

    let client = Client::new();
    let service_registry = data.get_ref();

    if let Some(service_url) = service_registry.services.get(&service_name) {
        // Construct the full URL
        let forward_url = format!("{}/{}", service_url, endpoint);
        log::debug!("Forwarding request to {}", forward_url);


        // Initialize the client request
        let mut client_req = client.request(req.method().clone(), &forward_url);

        // Copy the headers
        for (key, value) in req.headers().iter().filter(|(key, _)| !EXCLUDE_HEADERS.contains(key.as_str()))  {
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
            .map_err(Error::from)?;

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

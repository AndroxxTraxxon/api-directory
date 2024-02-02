use crate::{api_services::db::ApiServiceRepository, database::Database};
use actix_web::{web, Error, HttpRequest, HttpResponse, Result};
use futures_util::stream::TryStreamExt;
use reqwest::Client;

const EXCLUDE_HEADERS: &[&str] = &["host"];

pub async fn forward(
    req: HttpRequest,
    payload: web::Payload,
    db: web::Data<Database>,
) -> Result<HttpResponse> {

    let path = req.path().to_string();
    let segments: Vec<&str> = path.splitn(4, '/').collect();

    if segments.len() != 4 {
        return Ok(HttpResponse::BadRequest().finish());
    }
    let api_name = String::from(segments[1]);
    let version = String::from(segments[2]);
    let endpoint = String::from(segments[3]);

    log::debug!("Forwarding request to service [{}] version [{}] at endpoint {}", api_name, version, endpoint);

    let client = Client::new();

    if let Some(service) = Database::get_service_by_name_and_version(&db, api_name, version).await {
        // Construct the full URL
        log::debug!("Configured Forward URL: {}", service.forward_url);
        let forward_url = format!("{}/{}", service.forward_url, endpoint);
        log::debug!("Forwarding request to {}", forward_url);


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

use actix_web::{web, HttpRequest, HttpResponse, Result, HttpMessage, Error};
use reqwest::{Client, header};
use std::sync::Arc;
use futures::{StreamExt, TryStreamExt};
use crate::services::types::ServiceRegistry;


pub async fn forward(
    web::Path((service_name, endpoint)): web::Path<(String, String)>,
    req: HttpRequest,
    mut payload: web::Payload,
    data: web::Data<Arc<ServiceRegistry>>,
) -> Result<HttpResponse> {
    let client = Client::new();
    let service_registry = data.get_ref();

    if let Some(service_url) = service_registry.services.get(&service_name) {
        // Construct the full URL
        let forward_url = format!("{}{}", service_url, req.match_info().query("tail"));

        // Initialize the client request
        let mut client_req = client.request(req.method().clone(), &forward_url);

        // Copy the headers
        for (key, value) in req.headers().iter() {
            client_req = client_req.header(key.clone(), value.clone());
        }

        // Set additional headers for forwarding
        // ... (as before)

        // Stream the request body
        let body_stream = payload.try_fold(web::BytesMut::new(), |mut body, chunk| {
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

        // Convert the response into an Actix HttpResponse
        // ... (as before)
    } else {
        Ok(HttpResponse::NotFound().finish())
    }
}

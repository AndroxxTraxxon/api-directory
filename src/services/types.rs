use serde::{Deserialize, Serialize};
use std::sync::Mutex;


#[derive(Serialize, Deserialize)]
pub struct ApiConfig {
    pub name: String,
    pub endpoint: String,
    pub description: String,
    // Add more fields as necessary
}

// Shared state across endpoints
pub struct AppState {
    pub apis: Mutex<Vec<ApiConfig>>,
}

// This struct represents the service registry and would typically be part of your shared state.
pub struct ServiceRegistry {
    // Maps an endpoint name to a service URL.
    pub services: std::collections::HashMap<String, String>,
}
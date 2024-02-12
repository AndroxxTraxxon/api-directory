use serde::{Deserialize, Serialize};
use validator::Validate;
use surrealdb::sql::Thing;

#[derive(Debug, Serialize, Deserialize, Validate, Clone)]
pub struct ApiService {
    pub id: Option<Thing>,

    #[validate(length(min = 3))]
    pub api_name: String,

    #[validate(length(min = 3))]
    pub forward_url: String,

    pub active: bool,

    #[validate(length(min = 1))]
    pub version: String,

    #[validate]
    pub auth_details: Option<AuthDetails>,

    #[validate]
    pub rate_limiting: Option<RateLimiting>,

    #[validate(url)]
    pub health_check_url: Option<String>,

    #[validate(url)]
    pub documentation_url: Option<String>,

    #[validate]
    pub contact_info: ContactInfo,

    #[validate]
    pub sla: Option<SLADetails>,

    #[validate]
    pub security_requirements: Option<SecurityRequirements>,

    pub data_formats: Option<Vec<String>>, // might want a custom validation here to check for valid MIME types
    pub ip_whitelist: Option<Vec<String>>, // Custom validation could be added to ensure valid IP addresses
    pub ip_blacklist: Option<Vec<String>>, // Same as above for IP blacklist

    #[validate]
    pub caching_policy: Option<CachingPolicy>,

    #[validate(length(min = 1))]
    pub load_balancing_strategy: Option<String>,

    pub custom_headers: Option<Vec<String>>, // Custom validation might be needed based on your header requirements
    pub dependencies: Option<Vec<String>>,   // Validate based on your requirements for dependencies

    #[validate(length(min = 1))]
    pub environment: String,

    #[validate]
    pub deployment_info: Option<DeploymentInfo>,

    #[validate]
    pub error_handling: Option<ErrorHandling>,

    #[validate]
    pub metadata: Option<Vec<Metadata>>,
}

#[derive(Debug, Serialize, Deserialize, Validate, Clone)]
pub struct PartialApiServiceUpdate {
    #[validate(length(min = 1))]
    pub id: Option<String>,

    #[validate(length(min = 3))]
    pub api_name: Option<String>,

    #[validate(length(min = 3))]
    pub forward_url: Option<String>,

    pub active: Option<bool>,

    #[validate(length(min = 1))]
    pub version: Option<String>,

    #[validate]
    pub auth_details: Option<AuthDetails>,

    #[validate]
    pub rate_limiting: Option<RateLimiting>,

    #[validate(url)]
    pub health_check_url: Option<String>,

    #[validate(url)]
    pub documentation_url: Option<String>,

    #[validate]
    pub contact_info: Option<ContactInfo>,

    #[validate]
    pub sla: Option<SLADetails>,

    #[validate]
    pub security_requirements: Option<SecurityRequirements>,

    pub data_formats: Option<Vec<String>>, // might want a custom validation here to check for valid MIME type>s
    pub ip_whitelist: Option<Vec<String>>, // Custom validation could be added to ensure valid IP addresse>s
    pub ip_blacklist: Option<Vec<String>>, // Same as above for IP blacklis>t

    #[validate]
    pub caching_policy: Option<CachingPolicy>,

    #[validate(length(min = 1))]
    pub load_balancing_strategy: Option<String>,

    pub custom_headers: Option<Vec<String>>, // Custom validation might be needed based on your header requirement>
    pub dependencies: Option<Vec<String>>,   // Validate based on your requirements for dependencies

    #[validate(length(min = 1))]
    pub environment: Option<String>,

    #[validate]
    pub deployment_info: Option<DeploymentInfo>,

    #[validate]
    pub error_handling: Option<ErrorHandling>,

    #[validate]
    pub metadata: Option<Vec<Metadata>>,
}

#[derive(Debug, Serialize, Deserialize, Validate, Clone)]
pub struct AuthDetails {
    #[validate(length(min = 1))]
    pub method: String,

    #[validate(length(min = 1))]
    pub required_headers: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Validate, Clone)]
pub struct RateLimiting {
    #[validate(range(min = 1))]
    pub requests: u64,

    #[validate(length(min = 1))]
    pub interval: String,
}

#[derive(Debug, Serialize, Deserialize, Validate, Clone)]
pub struct ContactInfo {
    #[validate(length(min = 1))]
    pub team: String,

    #[validate(email)]
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize, Validate, Clone)]
pub struct SLADetails {
    #[validate(range(min = 0, max = 100))]
    pub uptime_percentage: f64,

    #[validate(range(min = 1))]
    pub response_time_ms: u64,
}

#[derive(Debug, Serialize, Deserialize, Validate, Clone)]
pub struct SecurityRequirements {
    pub protocols: Vec<String>, // Custom validation for valid protocols
    pub compliance_standards: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Validate, Clone)]
pub struct CachingPolicy {
    pub enabled: bool,

    #[validate(length(min = 1))]
    pub duration: String,
}

#[derive(Debug, Serialize, Deserialize, Validate, Clone)]
pub struct DeploymentInfo {
    #[validate(length(min = 1))]
    pub platform: String,

    pub container_info: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Validate, Clone)]
pub struct ErrorHandling {
    pub status_codes: Vec<u16>, // Validate for valid HTTP status codes

    pub custom_payloads: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Validate, Clone)]
pub struct Metadata {
    #[validate(length(min = 1))]
    pub key: String,

    #[validate(length(min = 1))]
    pub value: String,
}

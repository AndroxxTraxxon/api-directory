use actix_web::{
    delete, get, patch, post, put,
    web::{scope, to, Data, Json, Path, ServiceConfig},
    HttpRequest, HttpResponse,
};
use serde::Deserialize;

use crate::database::Database;
use crate::errors::{unknown_resource_error, Result};
use crate::{auth::web::validate_jwt, errors::GatewayError};

use super::models::{
    DbApiRole, WebApiRole, WebRequestApiService, WebRequestPartialApiService, WebResponseApiService,
};
use super::repo::{ApiServiceRepository, RoleRepository};

// Intermediate function to configure services
pub fn service_setup(cfg: &mut ServiceConfig) {
    cfg.service(
        scope("/cfg/v1/api-services")
            .service(list_services)
            .service(add_service)
            .service(patch_service)
            // .service(http::update_service)
            .service(get_service_by_name_and_version)
            .service(delete_service)
            .default_service(to(unknown_resource_error)),
    )
    .service(
        scope("/cfg/v1/api-roles")
            .service(list_roles)
            .service(add_role)
            .service(rename_role)
            .service(get_role_by_namespace_and_name)
            .service(delete_role),
    );
}

#[get("/")]
async fn list_services(
    req: HttpRequest,
    repo: Data<Database>,
) -> Result<Json<Vec<WebResponseApiService>>> {
    validate_jwt(&req, Some(&vec!["Gateway::Admin", "Gateway::BasicMember"]))?;
    let api_services = Database::list_services(&repo).await?;
    let api_services: Vec<WebResponseApiService> = api_services
        .iter()
        .map(|service| WebResponseApiService::from(service))
        .collect();
    Ok(Json(api_services))
}

#[derive(Deserialize)]
struct ApiRoleQualifiedNamePath {
    pub namespace: String,
    pub name: String,
}

#[get("/{namespace}/{name}")]
async fn get_role_by_namespace_and_name(
    req: HttpRequest,
    path_params: Path<ApiRoleQualifiedNamePath>,
    repo: Data<Database>,
) -> Result<Json<WebApiRole>> {
    validate_jwt(&req, Some(&vec!["Gateway::Admin", "Gateway::BasicMember"]))?;
    let parsed_path = path_params.into_inner();
    let namespace = parsed_path.namespace;
    let name = parsed_path.name;
    let found_role = Database::find_role(&repo, &namespace, &name).await?;
    Ok(Json(WebApiRole::from(&found_role)))
}

#[derive(Deserialize)]
struct ApiServiceNamedVersionPath {
    pub api_name: String,
    pub version: String,
}

#[get("/{api_name}/{version}")]
async fn get_service_by_name_and_version(
    req: HttpRequest,
    path_params: Path<ApiServiceNamedVersionPath>,
    repo: Data<Database>,
) -> Result<Json<WebResponseApiService>> {
    validate_jwt(&req, Some(&vec!["Gateway::Admin", "Gateway::BasicMember"]))?;
    let parsed_path = path_params.into_inner();
    let api_name = parsed_path.api_name;
    let version = parsed_path.version;
    let found_service = Database::get_service_with_roles(&repo, &api_name, &version).await?;
    Ok(Json(WebResponseApiService::from(&found_service)))
}

#[post("/")]
async fn add_service(
    req: HttpRequest,
    service: Json<WebRequestApiService>,
    repo: Data<Database>,
) -> Result<Json<WebResponseApiService>> {
    validate_jwt(&req, Some(&vec!["Gateway::Admin"]))?;
    let service_to_add = service.into_inner();
    let roles: Vec<WebApiRole> = Vec::<WebApiRole>::from(&service_to_add);
    let mut db_roles: Vec<DbApiRole> = Vec::new();
    for role in roles.iter() {
        match &role.id {
            Some(_) => db_roles.push(role.into()),
            None => match Database::find_role(&repo, &role.namespace, &role.name).await {
                Ok(role) => db_roles.push(role),

                Err(GatewayError::NotFound(_t, _m)) => {
                    db_roles.push(Database::add_role(&repo, &role).await?.into())
                }
                Err(other_error) => {
                    return Err(other_error);
                }
            },
        };
    }

    let created_service =
        Database::add_service(&repo, &(&service_to_add).into(), &db_roles).await?;
    Ok(Json(WebResponseApiService::from(&created_service)))
}

#[derive(Deserialize)]
struct ApiServiceIdPath {
    pub service_id: String,
}

#[patch("/{service_id}")]
async fn patch_service(
    req: HttpRequest,
    path_params: Path<ApiServiceIdPath>,
    service: Json<WebRequestPartialApiService>,
    repo: Data<Database>,
) -> Result<Json<WebResponseApiService>> {
    validate_jwt(&req, Some(&vec!["Gateway::Admin"]))?;
    let service_id = path_params.into_inner().service_id;
    let patched_service =
        Database::update_service(&repo, &service_id, &service.into_inner().into()).await?;
    Ok(Json(WebResponseApiService::from(&patched_service)))
}

#[delete("/{service_id}")]
async fn delete_service(
    req: HttpRequest,
    path_params: Path<ApiServiceIdPath>,
    repo: Data<Database>,
) -> Result<HttpResponse> {
    validate_jwt(&req, Some(&vec!["Gateway::Admin"]))?;
    let service_id = path_params.into_inner().service_id;
    Database::delete_service(&repo, service_id.as_str()).await?;
    Ok(HttpResponse::NoContent().finish())
}

/**
 * Role management
 */

#[get("/")]
async fn list_roles(req: HttpRequest, repo: Data<Database>) -> Result<Json<Vec<WebApiRole>>> {
    validate_jwt(&req, Some(&vec!["Gateway::Admin", "Gateway::BasicMember"]))?;
    let api_roles = Database::list_roles(&repo).await?;
    let api_roles = api_roles.iter().map(Into::into).collect();
    Ok(Json(api_roles))
}

#[post("/")]
async fn add_role(
    req: HttpRequest,
    role: Json<WebApiRole>,
    repo: Data<Database>,
) -> Result<Json<WebApiRole>> {
    validate_jwt(&req, Some(&vec!["Gateway::Admin"]))?;
    let created_role = Database::add_role(&repo, &role.into_inner()).await?;
    Ok(Json(WebApiRole::from(&created_role)))
}

#[derive(Deserialize)]
struct ApiRoleIdPath {
    pub role_id: String,
}

#[put("/{role_id}")]
async fn rename_role(
    req: HttpRequest,
    path_params: Path<ApiRoleIdPath>,
    role: Json<WebApiRole>,
    repo: Data<Database>,
) -> Result<Json<WebApiRole>> {
    validate_jwt(&req, Some(&vec!["Gateway::Admin"]))?;
    let role_id = path_params.into_inner().role_id;
    let updated_role = Database::rename_role(&repo, &role_id, &role.into_inner()).await?;
    Ok(Json(WebApiRole::from(&updated_role)))
}

#[delete("/{role_id}")]
async fn delete_role(
    req: HttpRequest,
    path_params: Path<ApiRoleIdPath>,
    repo: Data<Database>,
) -> Result<HttpResponse> {
    validate_jwt(&req, Some(&vec!["Gateway::Admin"]))?;
    let role_id = path_params.into_inner().role_id;
    Database::delete_role(&repo, role_id.as_str()).await?;
    Ok(HttpResponse::NoContent().finish())
}

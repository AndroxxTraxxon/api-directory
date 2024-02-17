use serde::Deserialize;
use actix_web::{
    get, patch, post,
    web::{scope, Data, Json, Path, ServiceConfig},
    HttpRequest,
};

use super::{
    models::{GatewayUser, PartialGatewayUserUpdate},
    repo::UserRepository,
};

use crate::auth::rest::validate_jwt_for_scopes;
use crate::errors::{Result};
use crate::database::Database;

// Intermediate function to configure services
pub fn web_setup(cfg: &mut ServiceConfig) {
    cfg.service(
        scope("/cfg/v1")
            .service(list_users)
            .service(register_user)
            .service(user_detail)
            .service(update_user),
    );
}

#[get("/users")]
async fn list_users(req: HttpRequest, repo: Data<Database>) -> Result<Json<Vec<GatewayUser>>> {
    validate_jwt_for_scopes(&req, &vec!["admin"])?;
    let user_list = Database::list_users(&repo).await?;
    Ok(Json(user_list))
}

#[post("/users")]
async fn register_user(req: HttpRequest, repo: Data<Database>, user_json: Json<GatewayUser>) -> Result<Json<GatewayUser>> {
    validate_jwt_for_scopes(&req, &vec!["admin"])?;
    let user_data = user_json.into_inner();
    let registered_user = Database::register_user(&repo, user_data).await?;
    Ok(Json(registered_user))
}

#[derive(Deserialize)]
struct UserIdPathParams {
    pub user_id: String,
}

#[get("/users/{user_id}")]
async fn user_detail(
    req: HttpRequest,
    repo: Data<Database>,
    path_params: Path<UserIdPathParams>,
) -> Result<Json<GatewayUser>> {
    validate_jwt_for_scopes(&req, &vec!["admin", "user-readonly"])?;
    let user_id = path_params.into_inner().user_id;
    let user = Database::user_detail(&repo, &user_id).await?;
    Ok(Json(user))
}

#[patch("/users/{user_id}")]
async fn update_user(
    req: HttpRequest,
    repo: Data<Database>,
    path_params: Path<UserIdPathParams>,
    user_form: Json<PartialGatewayUserUpdate>,
) -> Result<Json<GatewayUser>> {
    validate_jwt_for_scopes(&req, &vec!["admin"])?;
    let user_id = path_params.into_inner().user_id;
    let user = user_form.into_inner();
    let updated_user = Database::update_user(&repo, &user_id, &user).await?;
    Ok(Json(updated_user))
}

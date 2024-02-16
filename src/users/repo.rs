use std::time::SystemTime;

use actix_web::web::Data;
use async_trait::async_trait;
use serde::Serialize;
use surrealdb::sql::{Id, Thing};

use super::{
    errors::UserError,
    models::{GatewayUser, PartialGatewayUserUpdate},
};
use crate::auth::models::PasswordResetRequest;
use crate::database::Database;

const USER_TABLE: &str = "gateway_user";
const PASSWORD_RESET_TABLE: &str = "password_reset_request";

#[async_trait]
pub trait UserRepository {
    async fn register_user(
        repo: &Data<Database>,
        user: GatewayUser,
    ) -> Result<GatewayUser, UserError>;

    async fn user_detail(repo: &Data<Database>, user_id: &String)
        -> Result<GatewayUser, UserError>;

    async fn update_user(
        _repo: &Data<Database>,
        _user_id: &String,
        _user: &PartialGatewayUserUpdate,
    ) -> Result<GatewayUser, UserError>;

    async fn list_users(repo: &Data<Database>) -> Result<Vec<GatewayUser>, UserError>;
}

#[derive(Serialize)]
struct _UserIdQueryParams<'_a, '_b> {
    pub table: &'_a str,
    pub user_id: &'_b String,
}

pub async fn setup_user_table(repo: &Database) -> Result<(), String> {
    repo.define_index(
        USER_TABLE,
        "usernameIndex",
        vec!["username"],
        Some("UNIQUE"),
    )
    .await?;
    Ok(())
}

#[async_trait]
impl UserRepository for Database {
    async fn register_user(
        repo: &Data<Database>,
        new_user: GatewayUser,
    ) -> Result<GatewayUser, UserError> {
        // First, validate the GatewayUser
        // new_user.validate().map_err(|e| UserError::ValidationError(e.to_string()))?;

        // Insert the GatewayUser into the database
        let inserted_user: GatewayUser = repo
            .db
            .create(USER_TABLE)
            .content(new_user)
            .await
            .map_err(|e| UserError::DatabaseError(e.to_string()))?
            .remove(0);

        // Generate a PasswordResetRequest for the user
        let now_ts = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if let Some(Thing {
            tb: _,
            id: Id::String(ref user_id),
        }) = inserted_user.id.clone()
        {
            // let user_id = user_id.clone();
            let password_reset_request = PasswordResetRequest {
                id: None,
                user_id: user_id.clone(), // Assuming `id` is some form of unique identifier
                used: false,
                expires_at: now_ts + (24 * 60 * 60), // expires in 1 day
            };

            let password_reset: PasswordResetRequest = repo
                .db
                .create(PASSWORD_RESET_TABLE)
                .content(password_reset_request)
                .await
                .map_err(|e| UserError::DatabaseError(e.to_string()))?
                .remove(0);
            log::info!(
                "Created Password Reset request {} for user {}",
                password_reset.id.unwrap().id,
                user_id.clone()
            );
            Ok(inserted_user)
        } else {
            Err(UserError::DatabaseError(format!(
                "Unexpected User Id: {:?}",
                inserted_user.id.unwrap().id
            )))
        }
    }

    async fn user_detail(
        repo: &Data<Database>,
        user_id: &String,
    ) -> Result<GatewayUser, UserError> {
        let mut response = repo
            .db
            .query(
                "SELECT * FROM type::table($table) \
                WHERE id = $user_id",
            )
            .bind(_UserIdQueryParams {
                table: USER_TABLE,
                user_id,
            })
            .await
            .map_err(|e| UserError::DatabaseError(e.to_string()))?;
        let query_result: Option<GatewayUser> = response
            .take(0)
            .map_err(|e| UserError::DatabaseError(e.to_string()))?;

        match query_result {
            Some(user) => Ok(user),
            None => Err(UserError::UserNotFound(String::from(
                "Could not authenticate with the provided username and password",
            ))),
        }
    }

    async fn update_user(
        _repo: &Data<Database>,
        _user_id: &String,
        _user: &PartialGatewayUserUpdate,
    ) -> Result<GatewayUser, UserError> {
        Err(UserError::NotImplemented(String::from("update_user")))
    }

    async fn list_users(_repo: &Data<Database>) -> Result<Vec<GatewayUser>, UserError> {
        Err(UserError::NotImplemented(String::from("list_users")))
    }
}

use actix_web::web::Data;
use async_trait::async_trait;
use chrono::{Duration, Utc};
use serde::Serialize;

use crate::database::Database;

use super::{
    errors::UserError,
    models::{GatewayUser, PartialGatewayUserUpdate, PasswordResetRequest},
};
use surrealdb::sql::Id;

const USER_TABLE: &str = "gateway_user";
const PASSWORD_RESET_TABLE: &str = "password_reset_request";

#[async_trait]
pub trait UserRepository {
    async fn register_user(
        repo: &Data<Database>,
        user: GatewayUser,
    ) -> Result<GatewayUser, UserError>;

    async fn authenticate_user(
        repo: &Data<Database>,
        username: &String,
        password: &String,
    ) -> Result<bool, UserError>;

    async fn request_password_reset(
        repo: &Data<Database>,
        username: &String,
    ) -> Result<(), UserError>;

    async fn set_user_password_with_reset_token(
        repo: &Data<Database>,
        reset_token: &String,
        new_password: &String,
    ) -> Result<(), UserError>;

    async fn set_user_password(
        repo: &Data<Database>,
        username: &String,
        new_password: &String,
    ) -> Result<(), UserError>;

    async fn update_user(
        repo: &Data<Database>,
        user: &PartialGatewayUserUpdate,
    ) -> Result<GatewayUser, UserError>;

    async fn list_users(
        repo: &Data<Database>
    ) -> Result<Vec<GatewayUser>, UserError>;
}

#[derive(Serialize)]
struct _UserAuthenticationParams<'_a, '_b> {
    pub table: &'_a str,
    pub username: &'_b String,
    pub password: &'_b String,
}

pub async fn setup_user_table_events(
    repo: &Database
) -> Result<(), String>{
    let response = repo.db.query(format!("INFO FOR TABLE {}", USER_TABLE))
    .await
    .map_err(|e| e.to_string())?;
    dbg!("{:?}", response);

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
        if let Id::String(user_id) = inserted_user.id.clone().unwrap().id {
            // let user_id = user_id.clone();
            let password_reset_request = PasswordResetRequest {
                id: None,
                user_id: user_id.clone(), // Assuming `id` is some form of unique identifier
                used: false,
                expires_at: Utc::now() + Duration::days(3), // Example: expires in 1 day
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
                user_id
            );
        } else {
            return Err(UserError::DatabaseError(format!(
                "Unexpected User Id: {:?}",
                inserted_user.id.unwrap().id
            )));
        }

        // Return the inserted GatewayUser
        Ok(inserted_user)
    }

    async fn authenticate_user(
        repo: &Data<Database>,
        username: &String,
        password: &String,
    ) -> Result<bool, UserError> {
        let mut response = repo
            .db
            .query("\
                SELECT * FROM type::table($table) \
                WHERE username = $username \
                AND crypto::argon2::compare(password_hash, $password)\
            ")
            .bind(_UserAuthenticationParams {table: USER_TABLE, username, password })
            .await
            .map_err(|e| UserError::DatabaseError(e.to_string()))?;
        let query_result: Option<GatewayUser> = response.take(0)
            .map_err(|e| UserError::DatabaseError(e.to_string()))?;

        match query_result {
            Some(_) => Ok(true),
            None => Err(UserError::UserNotFound(String::from("Could not authenticate with the provided username and password")))
        }
    }

    // leveraging surrealdb argon2 implementation, which already hashes and salts passwords for ease of use
    // https://docs.surrealdb.com/docs/surrealql/functions/crypto#cryptoargon2generate
    async fn set_user_password_with_reset_token(
        _repo: &Data<Database>,
        _reset_token: &String,
        _new_password: &String,
    ) -> Result<(), UserError> {
        Err(UserError::NotImplemented(String::from(
            "set_user_password_with_reset_token",
        )))
    }

    async fn request_password_reset(
        _repo: &Data<Database>,
        _username: &String,
    ) -> Result<(), UserError> {
        Err(UserError::NotImplemented(String::from(
            "request_password_reset",
        )))
    }

    async fn set_user_password(
        _repo: &Data<Database>,
        _username: &String,
        _new_password: &String,
    ) -> Result<(), UserError> {
        Err(UserError::NotImplemented(String::from("set_user_password")))
    }

    async fn update_user(
        _repo: &Data<Database>,
        _user: &PartialGatewayUserUpdate,
    ) -> Result<GatewayUser, UserError> {
        Err(UserError::NotImplemented(String::from("update_user")))
    }

    async fn list_users(
        _repo: &Data<Database>,
    ) -> Result<Vec<GatewayUser>, UserError> {
        Err(UserError::NotImplemented(String::from("list_users")))
    }
}

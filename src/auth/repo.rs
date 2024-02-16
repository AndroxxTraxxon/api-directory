use actix_web::web::Data;
use async_trait::async_trait;
use chrono::{Duration, Utc};
use serde::Serialize;

use crate::database::Database;
use super::{
    errors::AuthError,
    models::PasswordResetRequest,
};
use crate::users::models::GatewayUser;

const USER_TABLE: &str = "gateway_user";
const PASSWORD_RESET_TABLE: &str = "password_reset_request";

#[async_trait]
pub trait UserAuthRepository {

    async fn authenticate_user(
        repo: &Data<Database>,
        username: &String,
        password: &String,
    ) -> Result<GatewayUser, AuthError>;

    async fn request_password_reset(
        repo: &Data<Database>,
        username: &String,
    ) -> Result<(), AuthError>;

    async fn set_user_password_with_reset_token(
        repo: &Data<Database>,
        reset_token: &String,
        new_password: &String,
    ) -> Result<(), AuthError>;

    async fn set_user_password(
        repo: &Data<Database>,
        username: &String,
        new_password: &String,
    ) -> Result<(), AuthError>;
}

#[derive(Serialize)]
struct _UserAuthenticationParams<'_a, '_b> {
    pub table: &'_a str,
    pub username: &'_b String,
    pub password: &'_b String,
}

pub async fn setup_reset_request_table(repo: &Database) -> Result<(), String> {
    repo.automate_created_date(PASSWORD_RESET_TABLE).await?;
    repo.automate_last_modified_date(PASSWORD_RESET_TABLE).await?;
    Ok(())
}

#[async_trait]
impl UserAuthRepository for Database {

    async fn authenticate_user(
        repo: &Data<Database>,
        username: &String,
        password: &String,
    ) -> Result<GatewayUser, AuthError> {
        let mut response = repo
            .db
            .query(
                "\
                SELECT * FROM type::table($table) \
                WHERE username = $username \
                AND crypto::argon2::compare(password_hash, $password)\
            ",
            )
            .bind(_UserAuthenticationParams {
                table: USER_TABLE,
                username,
                password,
            })
            .await
            .map_err(|e| AuthError::DatabaseError(e.to_string()))?;
        let query_result: Option<GatewayUser> = response
            .take(0)
            .map_err(|e| AuthError::DatabaseError(e.to_string()))?;

        match query_result {
            Some(user) => Ok(user),
            None => Err(AuthError::InvalidUsernameOrPassword(String::from(
                "Could not authenticate with the provided username and password",
            ))),
        }
    }

    // leveraging surrealdb argon2 implementation, which already hashes and salts passwords for ease of use
    // https://docs.surrealdb.com/docs/surrealql/functions/crypto#cryptoargon2generate
    async fn set_user_password_with_reset_token(
        _repo: &Data<Database>,
        _reset_token: &String,
        _new_password: &String,
    ) -> Result<(), AuthError> {
        Err(AuthError::NotImplemented(String::from(
            "set_user_password_with_reset_token",
        )))
    }

    async fn request_password_reset(
        _repo: &Data<Database>,
        _username: &String,
    ) -> Result<(), AuthError> {
        Err(AuthError::NotImplemented(String::from(
            "request_password_reset",
        )))
    }

    async fn set_user_password(
        _repo: &Data<Database>,
        _username: &String,
        _new_password: &String,
    ) -> Result<(), AuthError> {
        Err(AuthError::NotImplemented(String::from("set_user_password")))
    }
}

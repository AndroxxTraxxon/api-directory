use std::collections::BTreeMap;
use std::time;

use actix_web::web::Data;
use async_trait::async_trait;
use surrealdb::opt::PatchOp;
use surrealdb::sql::{Datetime, Id, Strand};

use super::models::PasswordResetRequest;
use crate::database::{Database, PASSWORD_RESET_TABLE, ROLE_MEMBER_TABLE, USER_TABLE};
use crate::errors::{GatewayError, Result};
use crate::users::models::{DbGatewayUserRecord, DbGatewayUserResponse};

const REQUEST_LIFETIME: u64 = 24 * 60 * 60;

#[async_trait]
pub trait UserAuthRepository {
    async fn authenticate_user(
        repo: &Data<Database>,
        username: &String,
        password: &String,
    ) -> Result<DbGatewayUserResponse>;

    async fn set_last_login(repo: &Data<Database>, user_id: &String) -> Result<()>;

    async fn request_password_reset(repo: &Data<Database>, username: &String) -> Result<()>;

    async fn set_user_password_with_reset_token(
        repo: &Data<Database>,
        reset_token: &String,
        username: &String,
        new_password: &String,
    ) -> Result<()>;

    async fn set_user_password(
        repo: &Data<Database>,
        user_id: &String,
        new_password: &String,
    ) -> Result<()>;
}

pub async fn setup_reset_request_table(repo: &Database) -> std::io::Result<()> {
    repo.automate_created_date(PASSWORD_RESET_TABLE).await?;
    repo.automate_last_modified_date(PASSWORD_RESET_TABLE)
        .await?;
    Ok(())
}

#[async_trait]
impl UserAuthRepository for Database {
    async fn authenticate_user(
        repo: &Data<Database>,
        username: &String,
        password: &String,
    ) -> Result<DbGatewayUserResponse> {
        let bind_params: BTreeMap<String, surrealdb::sql::Value> = [
            (
                "userTable".into(),
                surrealdb::sql::Value::Strand(Strand::from(USER_TABLE)),
            ),
            (
                "username".into(),
                surrealdb::sql::Value::Strand(Strand::from(username.clone())),
            ),
            (
                "password".into(),
                surrealdb::sql::Value::Strand(Strand::from(password.clone())),
            ),
        ]
        .into();
        let mut response = repo
            .db
            .query(format!(
                "\
                SELECT *, ->{}->role.* as roles FROM type::table($userTable) \
                WHERE username = $username \
                AND password_hash IS NOT NONE
                AND crypto::argon2::compare(password_hash, $password)\
            ",
                ROLE_MEMBER_TABLE
            ))
            .bind(bind_params)
            .await
            .map_err(Into::<GatewayError>::into)?;
        log::info!("Queried user... converting...");
        let query_result: Option<DbGatewayUserResponse> =
            response.take(0).map_err(Into::<GatewayError>::into)?;
        query_result.ok_or(GatewayError::InvalidUsernameOrPassword(String::from(
            "Could not authenticate with the provided username and password",
        )))
    }

    async fn set_last_login(repo: &Data<Database>, user_id: &String) -> Result<()> {
        let _: Option<DbGatewayUserRecord> = repo
            .db
            .update((USER_TABLE, user_id))
            .patch(PatchOp::replace("/last_login", Datetime::default()))
            .await
            .map_err(Into::<GatewayError>::into)?;
        Ok(())
    }

    async fn set_user_password(
        repo: &Data<Database>,
        user_id: &String,
        new_password: &String,
    ) -> Result<()> {
        log::debug!(
            "Hashing password [ {} ] for user [ {} ]",
            new_password,
            user_id
        );

        let pass_hash: Option<String> = repo
            .db
            .query("RETURN crypto::argon2::generate($password)")
            .bind(("password", new_password))
            .await
            .map_err(Into::<GatewayError>::into)?
            .take(0)
            .map_err(Into::<GatewayError>::into)?;

        let pass_hash = pass_hash.ok_or(GatewayError::DatabaseError(String::from(
            "Unable to hash password",
        )))?;

        log::debug!("New Password Hash: [ {} ] ", pass_hash);
        let now = Datetime::default();
        let _: DbGatewayUserRecord = repo
            .db
            .update((USER_TABLE, user_id))
            .patch(PatchOp::replace("/password_hash", &pass_hash))
            .patch(PatchOp::replace("/password_reset_at", &now))
            .patch(PatchOp::replace("/last_modified_date", &now))
            .await
            .map_err(Into::<GatewayError>::into)?
            .ok_or(GatewayError::NotFound(
                String::from("User"),
                String::from("Unknown User"),
            ))?;

        Ok(())
    }

    // leveraging surrealdb argon2 implementation, which already hashes and salts passwords for ease of use
    // https://docs.surrealdb.com/docs/surrealql/functions/crypto#cryptoargon2generate
    async fn set_user_password_with_reset_token(
        repo: &Data<Database>,
        reset_token: &String,
        username: &String,
        new_password: &String,
    ) -> Result<()> {
        let reset_request: PasswordResetRequest = repo
            .db
            .select((PASSWORD_RESET_TABLE, reset_token))
            .await
            .map_err(Into::<GatewayError>::into)?
            .ok_or(GatewayError::BadRequest(
                "Invalid Password Reset Request".to_string(),
            ))?;

        if reset_request.used {
            return Err(GatewayError::BadRequest(
                "Password Reset Request has already been fulilled.".to_string(),
            ));
        }

        let now = time::SystemTime::now()
            .duration_since(time::UNIX_EPOCH)
            .map_err(|e| GatewayError::SystemError(e.to_string()))?
            .as_secs();
        if reset_request.expires_at < now || reset_request.used {
            return Err(GatewayError::BadRequest(
                "Password Reset Request has expired.".to_string(),
            ));
        }

        let user: DbGatewayUserRecord = repo
            .db
            .select((USER_TABLE, &reset_request.user_id))
            .await
            .map_err(Into::<GatewayError>::into)?
            .ok_or(GatewayError::BadRequest(
                "Invalid Password Reset Request".to_string(),
            ))?;

        if user.username.ne(username) {
            return Err(GatewayError::BadRequest(
                "Invalid Password Reset Request".to_string(),
            ));
        }

        Database::set_user_password(repo, &reset_request.user_id, new_password).await?;
        repo.db
            .update::<Option<PasswordResetRequest>>((PASSWORD_RESET_TABLE, reset_token))
            .patch(PatchOp::replace("/used", true))
            .patch(PatchOp::replace("/last_modified", Datetime::default()))
            .await
            .map_err(Into::<GatewayError>::into)?;

        Ok(())
    }

    async fn request_password_reset(repo: &Data<Database>, username: &String) -> Result<()> {
        let bind_data: std::collections::BTreeMap<String, surrealdb::sql::Value> = [
            ("username".into(), username.clone().into()),
            ("table".into(), USER_TABLE.into()),
        ]
        .into();
        let mut response = repo
            .db
            .query(
                "SELECT * FROM type::table($table) \
                WHERE username = $username",
            )
            .bind(bind_data)
            .await
            .map_err(Into::<GatewayError>::into)?;

        let query_result: Option<DbGatewayUserRecord> =
            response.take(0).map_err(Into::<GatewayError>::into)?;

        let found_user: DbGatewayUserRecord = query_result.ok_or(GatewayError::NotFound(
            String::from("User"),
            String::from("Could not find user to request password reset"),
        ))?;
        let now = time::SystemTime::now()
            .duration_since(time::UNIX_EPOCH)
            .map_err(|e| GatewayError::SystemError(e.to_string()))?
            .as_secs();
        if let Id::String(user_id) = found_user.id.id {
            let response: Vec<PasswordResetRequest> = repo
                .db
                .create(PASSWORD_RESET_TABLE)
                .content(PasswordResetRequest {
                    id: None,
                    expires_at: now + REQUEST_LIFETIME,
                    user_id: user_id.clone(),
                    used: false,
                    last_modified: Datetime::default(),
                })
                .await
                .map_err(Into::<GatewayError>::into)?;

            let reset_request = response.get(0).ok_or(GatewayError::DatabaseError(
                "Unable to create Password Reset Request".to_string(),
            ))?;

            log::debug!(
                "New password reset {} created for user {}",
                reset_request.id.as_ref().unwrap().id.to_string(),
                user_id.clone()
            );
        } else {
            return Err(GatewayError::DatabaseError(
                "Unknown User ID Format for selected User".to_string(),
            ));
        }

        Ok(())
    }
}

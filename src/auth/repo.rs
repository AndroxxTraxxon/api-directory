use std::time;

use actix_web::web::Data;
use async_trait::async_trait;
use serde::Serialize;
use surrealdb::opt::PatchOp;
use surrealdb::sql::{Datetime, Id, Thing};

use super::models::PasswordResetRequest;
use crate::database::{Database, PASSWORD_RESET_TABLE, USER_TABLE};
use crate::errors::{GatewayError, Result};
use crate::users::models::GatewayUser;

const REQUEST_LIFETIME: u64 = 24 * 60 * 60;

#[async_trait]
pub trait UserAuthRepository {
    async fn authenticate_user(
        repo: &Data<Database>,
        username: &String,
        password: &String,
    ) -> Result<GatewayUser>;

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

#[derive(Serialize)]
struct _UserAuthenticationParams<'_a, '_b> {
    pub table: &'_a str,
    pub username: &'_b String,
    pub password: &'_b String,
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
    ) -> Result<GatewayUser> {
        let mut response = repo
            .db
            .query(
                "\
                SELECT * FROM type::table($table) \
                WHERE username = $username \
                AND password_hash IS NOT NONE
                AND crypto::argon2::compare(password_hash, $password)\
            ",
            )
            .bind(_UserAuthenticationParams {
                table: USER_TABLE,
                username,
                password,
            })
            .await
            .map_err(|e| GatewayError::DatabaseError(e.to_string()))?;
        let query_result: Option<GatewayUser> = response
            .take(0)
            .map_err(|e| GatewayError::DatabaseError(e.to_string()))?;
        query_result.ok_or(GatewayError::InvalidUsernameOrPassword(String::from(
            "Could not authenticate with the provided username and password",
        )))
    }

    async fn set_last_login(repo: &Data<Database>, user_id: &String) -> Result<()> {
        let _: Option<GatewayUser> = repo
            .db
            .update((USER_TABLE, user_id))
            .patch(PatchOp::replace("/last_login", Datetime::default()))
            .await
            .map_err(|e| GatewayError::DatabaseError(e.to_string()))?;
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
            .map_err(|e| GatewayError::DatabaseError(e.to_string()))?
            .take(0)
            .map_err(|e| GatewayError::DatabaseError(e.to_string()))?;

        let pass_hash = pass_hash.ok_or(GatewayError::DatabaseError(String::from(
            "Unable to hash password",
        )))?;

        log::debug!("New Password Hash: [ {} ] ", pass_hash);
        let now = Datetime::default();
        let _: GatewayUser = repo
            .db
            .update((USER_TABLE, user_id))
            .patch(PatchOp::replace("/password_hash", &pass_hash))
            .patch(PatchOp::replace("/password_reset_at", &now))
            .patch(PatchOp::replace("/last_modified_date", &now))
            .await
            .map_err(|e| GatewayError::DatabaseError(e.to_string()))?
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
            .map_err(|e| GatewayError::DatabaseError(e.to_string()))?
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

        let user: GatewayUser = repo
            .db
            .select((USER_TABLE, &reset_request.user_id))
            .await
            .map_err(|e| GatewayError::DatabaseError(e.to_string()))?
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
            .map_err(|e| GatewayError::DatabaseError(e.to_string()))?;

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
            .map_err(|e| GatewayError::DatabaseError(e.to_string()))?;

        dbg!(&response);

        let query_result: Option<GatewayUser> = response
            .take(0)
            .map_err(|e| GatewayError::DatabaseError(e.to_string()))?;

        dbg!(&query_result);
        let found_user: GatewayUser = query_result.ok_or(GatewayError::NotFound(
            String::from("User"),
            String::from("Could not find user to request password reset"),
        ))?;
        let now = time::SystemTime::now()
            .duration_since(time::UNIX_EPOCH)
            .map_err(|e| GatewayError::SystemError(e.to_string()))?
            .as_secs();
        if let Some(Thing {
            tb: _,
            id: Id::String(user_id),
        }) = found_user.id
        {
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
                .map_err(|e| GatewayError::DatabaseError(e.to_string()))?;

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

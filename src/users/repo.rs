use std::time::SystemTime;

use actix_web::web::Data;
use async_trait::async_trait;
use serde::Serialize;
use serde_json::{to_value, Value};
use surrealdb::opt::PatchOp;
use surrealdb::sql::{Datetime, Id, Thing};

use super::models::{GatewayUser, PartialGatewayUserUpdate};
use crate::auth::models::PasswordResetRequest;
use crate::database::Database;
use crate::errors::{GatewayError, Result};

pub const USER_TABLE: &str = "gateway_user";
const PASSWORD_RESET_TABLE: &str = "password_reset__request";

#[async_trait]
pub trait UserRepository {
    async fn register_user(repo: &Data<Database>, user: PartialGatewayUserUpdate) -> Result<GatewayUser>;

    async fn user_detail(repo: &Data<Database>, user_id: &String) -> Result<GatewayUser>;

    async fn update_user(
        repo: &Data<Database>,
        user_id: &String,
        user: &PartialGatewayUserUpdate,
    ) -> Result<GatewayUser>;

    async fn list_users(repo: &Data<Database>) -> Result<Vec<GatewayUser>>;
}

#[derive(Serialize)]
struct _UserIdQueryParams<'_a, '_b> {
    pub table: &'_a str,
    pub user_id: &'_b String,
}

pub async fn setup_user_table(repo: &Database) -> std::io::Result<()> {
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
    async fn register_user(repo: &Data<Database>, new_user: PartialGatewayUserUpdate) -> Result<GatewayUser> {
        // First, validate the GatewayUser
        // new_user.validate().map_err(|e| GatewayError::ValidationError(e.to_string()))?;

        // Insert the GatewayUser into the database
        let inserted_user: GatewayUser = repo
            .db
            .create(USER_TABLE)
            .content(new_user)
            .await
            .map_err(|e| GatewayError::DatabaseError(e.to_string()))?
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
                last_modified: Datetime::default(),
            };

            let password_reset: PasswordResetRequest = repo
                .db
                .create(PASSWORD_RESET_TABLE)
                .content(password_reset_request)
                .await
                .map_err(|e| GatewayError::DatabaseError(e.to_string()))?
                .remove(0);
            log::info!(
                "Created Password Reset request {} for user {}",
                password_reset.id.unwrap().id,
                user_id.clone()
            );
            Ok(inserted_user)
        } else {
            Err(GatewayError::DatabaseError(format!(
                "Unexpected User Id: {:?}",
                inserted_user.id.unwrap().id
            )))
        }
    }

    async fn user_detail(repo: &Data<Database>, user_id: &String) -> Result<GatewayUser> {
        repo.db
            .select((USER_TABLE, user_id))
            .await
            .map_err(|e| GatewayError::DatabaseError(e.to_string()))?
            .ok_or(GatewayError::NotFound(
                "User".to_string(),
                "Could not find a user with the specified ID".to_string(),
            ))
    }

    async fn update_user(
        repo: &Data<Database>,
        user_id: &String,
        user: &PartialGatewayUserUpdate,
    ) -> Result<GatewayUser> {
        // Serialize the PartialApiServiceUpdate struct to a serde_json Value
        let update_data: Value =
            to_value(user).map_err(|e| GatewayError::MissingData(e.to_string()))?; // Handle this unwrap more gracefully in production code

        if let Value::Object(fields) = update_data {
            // Start constructing the update query for the specific service ID
            let mut patch_request = repo
                .db
                .update((USER_TABLE, user_id))
                .patch(PatchOp::replace("/last_modified", Datetime::default()));

            // Iterate over the fields in the JSON object
            for (key, value) in fields {
                // Skip fields that are null or not provided in the partial update
                if !value.is_null() {
                    // Construct the JSON Pointer string
                    let prop_path = format!("/{}", key);

                    // Apply a patch operation for the current field
                    patch_request = patch_request.patch(PatchOp::replace(&prop_path, value));
                }
            }

            // Execute the update query
            match patch_request.await {
                Ok(updated_record) => match updated_record {
                    Some(value) => Ok(value),
                    None => Err(GatewayError::DatabaseError(String::from(
                        "Empty response from Database on update.",
                    ))),
                },
                Err(error) => Err(GatewayError::DatabaseError(error.to_string())),
            }
        } else {
            Err(GatewayError::MissingData(String::from(
                "Didn't understand the input data",
            ))) // The serialized update data is not an object, which shouldn't happen in correct implementations
        }
    }

    async fn list_users(repo: &Data<Database>) -> Result<Vec<GatewayUser>> {
        repo.db
            .select(USER_TABLE)
            .await
            .map_err(|e| GatewayError::DatabaseError(e.to_string()))
    }
}

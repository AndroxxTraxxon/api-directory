use std::collections::{BTreeMap, HashSet};
use std::time::SystemTime;

use actix_web::web::Data;
use async_trait::async_trait;
use serde::Serialize;
use serde_json::{to_value, Value};
use surrealdb::opt::PatchOp;
use surrealdb::sql::{Datetime, Thing};

use super::models::{
    DbGatewayUserRecord, DbGatewayUserRequest, DbGatewayUserResponse, DbPartialGatewayUserUpdate,
    DbRegisteredUser,
};
use crate::api_services::models::DbApiRole;
use crate::api_services::repo::RoleRepository;
use crate::auth::models::PasswordResetRequest;
use crate::database::{Database, ROLE_MEMBER_TABLE, USER_TABLE};
use crate::errors::{GatewayError, Result};

const PASSWORD_RESET_TABLE: &str = "password_reset_request";

#[async_trait]
pub trait UserRepository {
    async fn register_user(
        repo: &Data<Database>,
        user: DbGatewayUserRequest,
        roles: Vec<DbApiRole>,
    ) -> Result<DbGatewayUserResponse>;

    async fn user_detail(repo: &Data<Database>, user_id: &String) -> Result<DbGatewayUserResponse>;

    async fn update_user(
        repo: &Data<Database>,
        user_id: &String,
        user: DbPartialGatewayUserUpdate,
        roles: Option<Vec<DbApiRole>>,
    ) -> Result<DbGatewayUserResponse>;

    async fn list_users(repo: &Data<Database>) -> Result<Vec<DbGatewayUserResponse>>;

    async fn user_roles(repo: &Data<Database>, user_id: &Thing) -> Result<Vec<DbApiRole>>;
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

    repo.define_index(
        ROLE_MEMBER_TABLE,
        "roleMembershipIndex",
        vec!["in", "out"],
        Some("UNIQUE"),
    )
    .await?;
    repo.automate_created_date(ROLE_MEMBER_TABLE).await?;
    Ok(())
}

#[async_trait]
impl UserRepository for Database {
    async fn register_user(
        repo: &Data<Database>,
        new_user: DbGatewayUserRequest,
        roles: Vec<DbApiRole>,
    ) -> Result<DbGatewayUserResponse> {
        // First, validate the GatewayUser
        // new_user.validate().map_err(|e| GatewayError::ValidationError(e.to_string()))?;

        // Insert the GatewayUser into the database
        let inserted_user: DbRegisteredUser = repo
            .db
            .create(USER_TABLE)
            .content(new_user)
            .await
            .map_err(GatewayError::from)?
            .remove(0);

        let inserted_user: DbGatewayUserRecord = repo
            .db
            .select(inserted_user.id)
            .await
            .map_err(GatewayError::from)?
            .ok_or(GatewayError::DatabaseError(
                "Failed to fetch inserted user".to_string(),
            ))?;

        for role in roles.iter() {
            if let Some(role_id) = &role.id {
                repo.db
                    .select(role_id)
                    .await
                    .map_err(GatewayError::from)?
                    .ok_or(GatewayError::NotFound(
                        "Role".to_string(),
                        format!("{} could not be found", role_id),
                    ))?;
                repo.relate(role_id, &inserted_user.id, ROLE_MEMBER_TABLE, None)
                    .await?;
            }
        }

        // Generate a PasswordResetRequest for the user
        let now_ts = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let user_id = format!("{}", inserted_user.id);

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
            .map_err(Into::<GatewayError>::into)?
            .remove(0);
        log::info!(
            "Created Password Reset request {} for user {}",
            password_reset.id.unwrap().id,
            user_id.clone()
        );
        Ok((inserted_user, roles).into())
    }

    async fn user_detail(repo: &Data<Database>, user_id: &String) -> Result<DbGatewayUserResponse> {
        repo.db
            .select((USER_TABLE, user_id))
            .await
            .map_err(Into::<GatewayError>::into)?
            .ok_or(GatewayError::NotFound(
                "User".to_string(),
                "Could not find a user with the specified ID".to_string(),
            ))
    }

    async fn update_user(
        repo: &Data<Database>,
        user_id: &String,
        user: DbPartialGatewayUserUpdate,
        roles: Option<Vec<DbApiRole>>,
    ) -> Result<DbGatewayUserResponse> {
        // Serialize the DbPartialGatewayUserUpdate struct to a serde_json Value
        let user_id: Thing = ((USER_TABLE.to_string(), user_id.clone())).into();
        let mut intended_roles: Vec<DbApiRole> = Vec::new();
        if let Some(new_roles) = roles {
            for role in new_roles {
                if let Some(_) = role.id {
                    intended_roles.push(role.clone());
                } else {
                    intended_roles
                        .push(Database::find_role(repo, &role.namespace, &role.name).await?);
                }
            }
        }

        let mut new_role_ids: HashSet<Thing> = HashSet::new();
        for role in &intended_roles {
            new_role_ids.insert(role.id.clone().unwrap());
        }
        let existing_roles = Database::user_roles(repo, &user_id).await?;
        let mut existing_role_ids: HashSet<Thing> = HashSet::new();
        for role in existing_roles {
            existing_role_ids.insert(role.id.unwrap());
        }
        for role_to_remove in existing_role_ids.difference(&new_role_ids) {
            repo.unrelate(&user_id, role_to_remove, &ROLE_MEMBER_TABLE.to_string())
                .await?;
        }
        for role_to_add in new_role_ids.difference(&existing_role_ids) {
            repo.relate(&user_id, role_to_add, &ROLE_MEMBER_TABLE.to_string(), None)
                .await?;
        }

        let update_data: Value =
            to_value(user).map_err(|e| GatewayError::MissingData(e.to_string()))?; // Handle this unwrap more gracefully in production code

        if let Value::Object(fields) = update_data {
            // Start constructing the update query for the specific service ID
            let mut patch_request = repo
                .db
                .update(&user_id)
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
            let _update_result: DbGatewayUserRecord =
                patch_request.await.map_err(GatewayError::from)?.ok_or(
                    GatewayError::DatabaseError("Unable to update record".to_string()),
                )?;

            let result: Option<DbGatewayUserResponse> = repo
                .query_record(
                    format!(
                        "SELECT *, ->{}->role.* as roles FROM $user_id",
                        ROLE_MEMBER_TABLE
                    ),
                    Some::<(String, surrealdb::sql::Value)>(
                        (
                            "user_id".to_string(),
                            surrealdb::sql::Value::Thing(user_id.clone()),
                        )
                            .into(),
                    ),
                )
                .await?;

            result.ok_or(GatewayError::NotFound(
                String::from("API Service"),
                format!("An API Service could not be found at id {}", user_id),
            ))
        } else {
            Err(GatewayError::MissingData(String::from(
                "Didn't understand the input data",
            ))) // The serialized update data is not an object, which shouldn't happen in correct implementations
        }
    }

    async fn list_users(repo: &Data<Database>) -> Result<Vec<DbGatewayUserResponse>> {
        repo.db
            .query(format!(
                "SELECT *, ->{}->role.* as roles FROM {}",
                ROLE_MEMBER_TABLE, USER_TABLE
            ))
            .await
            .map_err(Into::<GatewayError>::into)?
            .take(0)
            .map_err(Into::<GatewayError>::into)
    }

    async fn user_roles(repo: &Data<Database>, user_id: &Thing) -> Result<Vec<DbApiRole>> {
        let bind_params: BTreeMap<String, surrealdb::sql::Value> = [(
            "user_id".to_string(),
            surrealdb::sql::Value::Thing(user_id.clone()),
        )]
        .into();
        let result: Option<Vec<DbApiRole>> = repo
            .query_record(
                format!(
                    "SELECT VALUE roles FROM (SELECT ->{}->role.* AS roles FROM $user_id)",
                    ROLE_MEMBER_TABLE
                ),
                Some(bind_params),
            )
            .await?;
        Ok(result.unwrap_or(Vec::new()))
    }
}

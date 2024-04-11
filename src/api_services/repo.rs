use std::collections::{BTreeMap, HashSet};

use super::models::{self, DbApiServiceRecord};
use crate::database::{Database, API_ROLE_TABLE, API_SERVICE_TABLE, AUTHORIZATIONS_TABLE};
use crate::errors::{GatewayError, Result};
use actix_web::web::Data;
use async_trait::async_trait;
use serde_json::{to_value, Value};
use surrealdb::method::Patch;
use surrealdb::sql::Thing;
use surrealdb::Result as dbResult;
use surrealdb::{opt::PatchOp, sql::Datetime};

#[async_trait]
pub trait ApiServiceRepository {
    async fn list_services(repo: &Data<Database>) -> Result<Vec<models::DbFullApiService>>;
    async fn get_service_with_roles(
        repo: &Data<Database>,
        api_name: &String,
        version: &String,
    ) -> Result<models::DbFullApiService>;
    async fn add_service(
        repo: &Data<Database>,
        new_service: &models::DbApiServiceRequest,
        roles: &Vec<models::DbApiRole>,
    ) -> Result<models::DbFullApiService>;
    async fn update_service(
        repo: &Data<Database>,
        service_id: &String,
        partial_update: &models::WebRequestPartialApiService,
    ) -> Result<models::DbFullApiService>;
    async fn delete_service(repo: &Data<Database>, service_name: &str) -> Result<()>;
}

pub async fn setup_service_table_events(repo: &Database) -> std::io::Result<()> {
    repo.define_index(
        API_SERVICE_TABLE,
        "serviceNamedVersionIndex",
        vec!["api_name", "version"],
        Some("UNIQUE"),
    )
    .await?;
    repo.automate_created_date(API_SERVICE_TABLE).await?;
    repo.automate_last_modified_date(API_SERVICE_TABLE).await?;

    repo.define_index(
        API_ROLE_TABLE,
        "namespacedRoleNameIndex",
        vec!["namespace", "name"],
        Some("UNIQUE"),
    )
    .await?;
    repo.automate_created_date(API_ROLE_TABLE).await?;
    repo.automate_last_modified_date(API_ROLE_TABLE).await?;

    repo.define_index(
        AUTHORIZATIONS_TABLE,
        "roleAuthorizationsIndex",
        vec!["in", "out"],
        Some("UNIQUE"),
    )
    .await?;
    repo.automate_created_date(AUTHORIZATIONS_TABLE).await?;
    Ok(())
}

#[async_trait]
impl ApiServiceRepository for Database {
    async fn list_services(repo: &Data<Database>) -> Result<Vec<models::DbFullApiService>> {
        repo.query_list::<models::DbFullApiService>(
            format!(
                "SELECT *, <-authorizes<-role.* as roles FROM {}",
                API_SERVICE_TABLE
            ),
            None::<String>,
        )
        .await
    }

    async fn get_service_with_roles(
        repo: &Data<Database>,
        api_name: &String,
        version: &String,
    ) -> Result<models::DbFullApiService> {
        let bind_vars: BTreeMap<&str, surrealdb::sql::Value> = [
            (
                "table",
                surrealdb::sql::Value::Strand(API_SERVICE_TABLE.into()),
            ),
            (
                "api_name",
                surrealdb::sql::Value::Strand(api_name.clone().into()),
            ),
            (
                "version",
                surrealdb::sql::Value::Strand(version.clone().into()),
            ),
        ]
        .into();
        let query_result: Vec<models::DbFullApiService> = repo
            .query_list(
                "\
            SELECT *, <-authorizes<-role.* as roles FROM type::table($table) \
            WHERE active = TRUE AND \
            api_name = $api_name AND \
            version = $version \
            LIMIT 1\
            ",
                Some(bind_vars),
            )
            .await?;

        Ok(query_result
            .first()
            .ok_or(GatewayError::NotFound(
                "API Service".into(),
                format!(
                    "No service named [{:?}] with version [{:?}] found.",
                    api_name, version
                ),
            ))?
            .clone())
    }

    async fn add_service(
        repo: &Data<Database>,
        new_service: &models::DbApiServiceRequest,
        roles: &Vec<models::DbApiRole>,
    ) -> Result<models::DbFullApiService> {
        let query_result: Vec<models::DbApiServiceRecord> = repo
            .db
            .create(API_SERVICE_TABLE)
            .content(new_service)
            .await
            .map_err(Into::<GatewayError>::into)?;

        let added_service = query_result.first().ok_or(GatewayError::DatabaseError(
            "Unable to insert record.".to_string(),
        ))?;

        for role in roles {
            if let Some(role_id) = &role.id {
                repo.relate(role_id, &added_service.id, AUTHORIZATIONS_TABLE, None)
                    .await?;
            }
        }
        Ok((added_service, roles).into())
    }

    async fn update_service(
        repo: &Data<Database>,
        service_id: &String,
        partial_update: &models::WebRequestPartialApiService,
    ) -> Result<models::DbFullApiService> {
        // Serialize the PartialApiServiceUpdate struct to a serde_json Value
        let service_db_id = Thing::from((API_SERVICE_TABLE.to_string(), service_id.clone()));
        let mut new_roles: Vec<models::DbApiRole> = Vec::new();
        for web_role in Vec::<models::WebApiRole>::from(partial_update).iter() {
            if let Some(_) = &web_role.id {
                new_roles.push(web_role.into());
            } else {
                match Database::find_role(&repo, &web_role.namespace, &web_role.name).await {
                    Ok(role) => new_roles.push(role),
                    Err(GatewayError::NotFound(_r, _m)) => {
                        new_roles.push(Database::add_role(&repo, web_role).await?)
                    }
                    Err(err) => return Err(err),
                }
            }
        }

        let mut new_role_ids: HashSet<Thing> = HashSet::new();
        for role in &new_roles {
            new_role_ids.insert(role.id.clone().unwrap());
        }
        let existing_roles = Database::roles_for_service(repo, &service_db_id).await?;
        let mut existing_role_ids: HashSet<Thing> = HashSet::new();
        for role in existing_roles {
            existing_role_ids.insert(role.id.unwrap());
        }
        for role_to_remove in existing_role_ids.difference(&new_role_ids) {
            repo.unrelate(
                role_to_remove,
                &service_db_id,
                &AUTHORIZATIONS_TABLE.to_string(),
            )
            .await?;
        }
        for role_to_add in new_role_ids.difference(&existing_role_ids) {
            repo.relate(
                role_to_add,
                &service_db_id,
                &AUTHORIZATIONS_TABLE.to_string(),
                None,
            )
            .await?;
        }

        let update_data: Value =
            to_value(partial_update).map_err(|e| GatewayError::MissingData(e.to_string()))?; // Handle this unwrap more gracefully in production code
        if let Value::Object(fields) = update_data {
            // Start constructing the update query for the specific service ID
            let mut patch_request: Patch<
                '_,
                surrealdb::engine::local::Db,
                Option<models::DbApiServiceRecord>,
            > = repo
                .db
                .update((API_SERVICE_TABLE, service_id))
                .patch(PatchOp::replace("/last_modified", Datetime::default()));

            let relationship_fields: Vec<String> = vec!["roles".into(), "role_namespaces".into()];
            // Iterate over the fields in the JSON object
            for (key, value) in fields {
                // Skip fields that are null or not provided in the partial update
                if !value.is_null() && !relationship_fields.contains(&key) {
                    // Construct the JSON Pointer string
                    let prop_path = format!("/{}", &key);

                    // Apply a patch operation for the current field
                    patch_request = patch_request.patch(PatchOp::replace(&prop_path, value));
                }
            }

            let patch_result = patch_request
                .await
                .map_err(Into::<GatewayError>::into)?
                .ok_or(GatewayError::DatabaseError(String::from(
                    "Empty response from Database on API Service patch update.",
                )))?;
            Ok((&patch_result, &new_roles).into())
        } else {
            Err(GatewayError::MissingData(String::from(
                "Didn't understand the input data",
            ))) // The serialized update data is not an object, which shouldn't happen in correct implementations
        }
    }

    async fn delete_service(repo: &Data<Database>, service_id: &str) -> Result<()> {
        repo.db
            .delete((API_SERVICE_TABLE, service_id))
            .await
            .or_else(|err| Err(err.into()))
            .and_then(
                |response: Option<models::DbApiServiceRecord>| match response {
                    Some(_) => Ok(()),
                    None => Err(GatewayError::DatabaseError(String::from(
                        "Unable to delete api service entry",
                    ))),
                },
            )
    }
}

#[async_trait]
pub trait RoleRepository {
    async fn list_roles(repo: &Data<Database>) -> Result<Vec<models::DbApiRole>>;
    async fn find_role(
        repo: &Data<Database>,
        namespace: &String,
        name: &String,
    ) -> Result<models::DbApiRole>;
    async fn roles_for_service(
        repo: &Data<Database>,
        service_id: &Thing,
    ) -> Result<Vec<models::DbApiRole>>;
    async fn get_namespaces(
        repo: &Data<Database>,
        namespaces: &Vec<String>,
    ) -> Result<Vec<models::DbApiRole>>;
    async fn add_role(
        repo: &Data<Database>,
        new_role: &models::WebApiRole,
    ) -> Result<models::DbApiRole>;
    async fn rename_role(
        repo: &Data<Database>,
        role_id: &String,
        role_update: &models::WebApiRole,
    ) -> Result<models::DbApiRole>;
    async fn delete_role(repo: &Data<Database>, service_name: &str) -> Result<()>;
}

#[async_trait]
impl RoleRepository for Database {
    async fn list_roles(repo: &Data<Database>) -> Result<Vec<models::DbApiRole>> {
        repo.db
            .select(API_ROLE_TABLE)
            .await
            .map_err(Into::<GatewayError>::into)
    }

    async fn find_role(
        repo: &Data<Database>,
        namespace: &String,
        name: &String,
    ) -> Result<models::DbApiRole> {
        let bind_data: std::collections::BTreeMap<String, surrealdb::sql::Value> = [
            ("role_ns".into(), namespace.clone().into()),
            ("role_name".into(), name.clone().into()),
        ]
        .into();
        // Requires an index to enforce uniqueness on api name and version
        let mut response = repo
            .db
            .query(format!(
                "\
                SELECT * FROM {} \
                WHERE namespace = $role_ns AND \
                name = $role_name \
                LIMIT 1\
            ",
                API_ROLE_TABLE
            ))
            .bind(bind_data)
            .await
            .map_err(Into::<GatewayError>::into)?;

        let query_result: dbResult<Option<models::DbApiRole>> = response.take(0);
        query_result
            .map_err(Into::<GatewayError>::into)?
            .ok_or(GatewayError::NotFound(
                String::from("API Service"),
                format!(
                    "No role named [{:?}] with in namespace [{:?}] found.",
                    name, namespace
                ),
            ))
    }

    async fn roles_for_service(
        repo: &Data<Database>,
        service_id: &Thing,
    ) -> Result<Vec<models::DbApiRole>> {
        let bind_params: BTreeMap<String, surrealdb::sql::Value> = [(
            "service_id".to_string(),
            surrealdb::sql::Value::Thing(service_id.clone()),
        )]
        .into();
        log::info!("Querying Roles for service {}", service_id);
        let roles: Vec<models::DbApiRole> = repo
            .query_record(
                format!(
                    "select value roles from (select <-{}<-role.* as roles from $service_id)",
                    AUTHORIZATIONS_TABLE
                ),
                Some(bind_params),
            )
            .await?
            .ok_or(GatewayError::NotFound(
                "Role".to_string(),
                format!("Could not find roles for {}", service_id).to_string(),
            ))?;

        Ok(roles)
    }

    async fn get_namespaces(
        repo: &Data<Database>,
        namespaces: &Vec<String>,
    ) -> Result<Vec<models::DbApiRole>> {
        // Convert namespaces into a format suitable for a "IN" query clause
        let bind_data: BTreeMap<String, surrealdb::sql::Value> = namespaces
            .iter()
            .enumerate()
            .map(|(i, ns)| {
                let key = format!("ns{}", i); // Create a unique key for each namespace
                (key, ns.clone().into()) // Convert the namespace string into a Value
            })
            .collect();

        // Generate placeholders for the "IN" clause
        let placeholders: Vec<String> = bind_data.keys().map(|k| format!("${}", k)).collect();

        // Construct the query
        let query = format!(
            "SELECT * FROM {} WHERE active = TRUE AND namespace IN ({})",
            API_ROLE_TABLE,
            placeholders.join(", ")
        );

        // Execute the query with binding parameters
        let mut response = repo
            .db
            .query(&query)
            .bind(bind_data)
            .await
            .map_err(Into::<GatewayError>::into)?;

        // Assuming the response can be directly converted into a Vec<ApiRole>
        let roles: Vec<models::DbApiRole> = response.take(0).map_err(Into::<GatewayError>::into)?;

        if roles.is_empty() {
            Err(GatewayError::NotFound(
                "API Roles".into(),
                "No roles found in the specified namespaces.".into(),
            ))
        } else {
            Ok(roles)
        }
    }

    async fn add_role(
        repo: &Data<Database>,
        new_role: &models::WebApiRole,
    ) -> Result<models::DbApiRole> {
        let create_result: surrealdb::Result<Vec<models::DbApiRole>> =
            repo.db.create(API_ROLE_TABLE).content(new_role).await;
        match create_result {
            Ok(records) => Ok(records.get(0).unwrap().clone()),
            Err(error) => Err(error.into()),
        }
    }

    async fn rename_role(
        repo: &Data<Database>,
        role_id: &String,
        role_update: &models::WebApiRole,
    ) -> Result<models::DbApiRole> {
        let update_result: Option<models::DbApiRole> = repo
            .db
            .update((API_ROLE_TABLE, role_id))
            .patch(PatchOp::replace("/last_modified", Datetime::default()))
            .patch(PatchOp::replace("/namespace", &role_update.namespace))
            .patch(PatchOp::replace("/name", &role_update.name))
            .await
            .map_err(GatewayError::from)?;

        update_result.ok_or(GatewayError::DatabaseError(format!(
            "Unable to update role {}",
            role_id
        )))
    }

    async fn delete_role(repo: &Data<Database>, role_id: &str) -> Result<()> {
        let auth_results: Vec<models::RelatedAuthorizations> = repo
            .query_list(
                "SELECT ->authorizes.id as authorizations FROM $role_id",
                Some((
                    "role_id".to_string(),
                    surrealdb::sql::Value::Thing((API_ROLE_TABLE, role_id).into()),
                )),
            )
            .await?;
        for role in auth_results {
            for role_auth in role.authorizations {
                let _: Option<DbApiServiceRecord> = repo
                    .db
                    .delete(role_auth)
                    .await
                    .map_err(Into::<GatewayError>::into)?;
            }
        }
        repo.db
            .delete((API_ROLE_TABLE, role_id))
            .await
            .or_else(|err| Err(GatewayError::DatabaseError(err.to_string())))
            .and_then(|response: Option<models::DbApiRole>| match response {
                Some(_) => Ok(()),
                None => Err(GatewayError::DatabaseError(String::from(
                    "Unable to delete api service entry",
                ))),
            })
    }
}

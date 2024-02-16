use super::models::{ApiService, PartialApiServiceUpdate};
use super::errors::ApiServiceError;
use crate::database::Database;
use actix_web::web::Data;
use async_trait::async_trait;
use serde::Serialize;
use serde_json::{to_value, Value};
use surrealdb::{opt::PatchOp, sql::Datetime};


const API_SERVICE_TABLE: &str = "service";

#[derive(Serialize)]
struct ServiceQueryParams<'a, 'b> {
    pub table: &'a str,
    pub api_name: &'b String,
    pub version: &'b String,
}

#[async_trait]
pub trait ApiServiceRepository {
    async fn get_all_services(repo: &Data<Database>) -> Result<Vec<ApiService>, ApiServiceError>;
    async fn get_service_by_name_and_version(
        repo: &Data<Database>,
        api_name: &String,
        version: &String,
    ) -> Result<ApiService, ApiServiceError>;
    async fn add_service(repo: &Data<Database>, new_service: &ApiService) -> Result<ApiService, ApiServiceError>;
    async fn update_service(
        repo: &Data<Database>,
        service_name: &String,
        updated_service: &ApiService,
    ) -> Result<ApiService, ApiServiceError>;
    async fn delete_service(repo: &Data<Database>, service_name: &str) -> Result<(), ApiServiceError>;
    async fn patch_service(
        repo: &Data<Database>,
        service_id: &String,
        partial_update: &PartialApiServiceUpdate,
    ) -> Result<ApiService, ApiServiceError>;
}

pub async fn setup_service_table_events(repo: &Database) -> Result<(), String> {
    repo.define_index(API_SERVICE_TABLE, "serviceNamedVersionIndex", vec!["api_name", "version"], Some("UNIQUE")).await?;
    repo.automate_created_date(API_SERVICE_TABLE).await?;
    repo.automate_last_modified_date(API_SERVICE_TABLE).await?;
    Ok(())
}

#[async_trait]
impl ApiServiceRepository for Database {
    async fn get_all_services(repo: &Data<Database>) -> Result<Vec<ApiService>, ApiServiceError> {
        repo.db
        .select(API_SERVICE_TABLE)
        .await
        .or_else(|err| {
            Err(ApiServiceError::DatabaseError(err.to_string()))
        })
    }

    async fn get_service_by_name_and_version(
        repo: &Data<Database>,
        api_name: &String,
        version: &String,
    ) -> Result<ApiService, ApiServiceError> {
        // Requires an index to enforce uniqueness on api name and version
        let mut response = repo.db
            .query(format!("\
                SELECT * FROM {} \
                WHERE active = TRUE AND \
                api_name = $api_name AND \
                version = $version \
                LIMIT 1\
            ", API_SERVICE_TABLE))
            .bind(ServiceQueryParams {table: API_SERVICE_TABLE, api_name, version })
            .await
            .map_err(|err| ApiServiceError::DatabaseError(err.to_string()))?;
    
        let query_result: Result<Option<ApiService>, surrealdb::Error> = response.take(0);
        query_result
            .map_err(|error| ApiServiceError::DatabaseError(error.to_string()))?
            .ok_or_else(|| ApiServiceError::ServiceNotFound(
                format!("No service named [{:?}] with version [{:?}] found.", api_name, version)
            ))
    }
    

    async fn add_service(repo: &Data<Database>, new_service: &ApiService) -> Result<ApiService, ApiServiceError> {
        let create_result: Result<Vec<ApiService>, _> =
            repo.db.create(API_SERVICE_TABLE).content(new_service).await;
        match create_result {
            Ok(records) => Ok(records.get(0).unwrap().clone()),
            Err(error) => Err(ApiServiceError::DatabaseError(error.to_string())),
        }
    }

    async fn patch_service(
        repo: &Data<Database>,
        service_id: &String,
        partial_update: &PartialApiServiceUpdate,
    ) -> Result<ApiService, ApiServiceError> {
        // Serialize the PartialApiServiceUpdate struct to a serde_json Value
        let update_data: Value = to_value(partial_update).unwrap(); // Handle this unwrap more gracefully in production code

        if let Value::Object(fields) = update_data {
            // Start constructing the update query for the specific service ID
            let mut patch_request = repo
                .db
                .update((API_SERVICE_TABLE, service_id))
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
                Ok(updated_record) => {
                    match updated_record {
                        Some(value) => Ok(value),
                        None => Err(ApiServiceError::DatabaseError(String::from("Empty response from Database on update.")))
                    }
                },
                Err(error) => Err(ApiServiceError::DatabaseError(error.to_string())),
            }
        } else {
            Err(ApiServiceError::MissingData(String::from("Didn't understand the input data"))) // The serialized update data is not an object, which shouldn't happen in correct implementations
        }
    }

    async fn update_service(
        _db: &Data<Database>,
        _service_id: &String,
        _updated_service: &ApiService,
    ) -> Result<ApiService, ApiServiceError> {
        Err(ApiServiceError::NotImplemented(String::from("update_service has not yet been implemented.")))
    }

    async fn delete_service(repo: &Data<Database>, service_id: &str) -> Result<(), ApiServiceError> {
        repo
        .db
        .delete((API_SERVICE_TABLE, service_id))
        .await
        .or_else(|err| Err(ApiServiceError::DatabaseError(err.to_string())))
        .and_then(|response: Option<PartialApiServiceUpdate>| {
            match response {
                Some(_) => Ok(()),
                None => Err(ApiServiceError::DatabaseError(String::from("Unable to delete api service entry")))
            }
        })
    }
}

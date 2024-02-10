use super::models::{ApiService, PartialApiServiceUpdate};
use crate::gw_database::Database;
use actix_web::web::Data;
use async_trait::async_trait;
use serde::Serialize;
use serde_json::{to_value, Value};
use surrealdb::{opt::PatchOp, sql::Datetime};

#[derive(Serialize)]
struct ServiceQueryParams {
    pub api_name: String,
    pub version: String,
}

#[async_trait]
pub trait ApiServiceRepository {
    async fn get_all_services(repo: &Data<Database>) -> Option<Vec<ApiService>>;
    async fn get_service_by_name_and_version(
        repo: &Data<Database>,
        api_name: String,
        version: String,
    ) -> Option<ApiService>;
    async fn add_service(repo: &Data<Database>, new_pizza: ApiService) -> Option<ApiService>;
    async fn update_service(
        repo: &Data<Database>,
        service_name: String,
        updated_service: ApiService,
    ) -> Option<ApiService>;
    async fn delete_service(repo: &Data<Database>, service_name: &str) -> Option<()>;
    async fn patch_service(
        repo: &Data<Database>,
        service_id: String,
        partial_update: PartialApiServiceUpdate,
    ) -> Option<ApiService>;
}

#[async_trait]
impl ApiServiceRepository for Database {
    async fn get_all_services(repo: &Data<Database>) -> Option<Vec<ApiService>> {
        let db_results = repo.db.select("service").await;
        match db_results {
            Ok(all_services) => Some(all_services),
            Err(_) => None,
        }
    }

    async fn get_service_by_name_and_version(
        repo: &Data<Database>,
        api_name: String,
        version: String,
    ) -> Option<ApiService> {
        repo.db
            .query(
                "\
            SELECT * FROM service \
            WHERE active = TRUE AND \
            api_name = $api_name AND \
            version = $version \
            LIMIT 1\
        ",
            )
            .bind(ServiceQueryParams { api_name, version })
            .await
            .ok() // Converts Result to Option, keeping Ok value and converting Err to None
            .and_then(|mut response| {
                let query_result = response.take(0);
                match query_result {
                    Ok(record) => record,
                    Err(message) => {
                        log::debug!("{:?}", message);
                        None
                    }
                }
            })
    }

    async fn add_service(repo: &Data<Database>, new_service: ApiService) -> Option<ApiService> {
        let create_result: Result<Vec<ApiService>, _> =
            repo.db.create("service").content(new_service).await;
        match create_result {
            Ok(records) => Some(records.get(0).unwrap().clone()),
            Err(_) => None,
        }
    }

    async fn patch_service(
        repo: &Data<Database>,
        service_id: String,
        partial_update: PartialApiServiceUpdate,
    ) -> Option<ApiService> {
        // Serialize the PartialApiServiceUpdate struct to a serde_json Value
        let update_data: Value = to_value(partial_update).unwrap(); // Handle this unwrap more gracefully in production code

        if let Value::Object(fields) = update_data {
            // Start constructing the update query for the specific service ID
            let mut patch_request = repo
                .db
                .update(("service", service_id.as_str()))
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
                Ok(updated_record) => updated_record,
                Err(_) => None,
            }
        } else {
            None // The serialized update data is not an object, which shouldn't happen in correct implementations
        }
    }

    async fn update_service(
        _db: &Data<Database>,
        _service_id: String,
        _updated_service: ApiService,
    ) -> Option<ApiService> {
        None
    }

    async fn delete_service(repo: &Data<Database>, service_id: &str) -> Option<()> {
        repo
        .db
        .delete(("service", service_id))
        .await
        .ok()
        .and_then(|response: Option<PartialApiServiceUpdate>| {
            match response {
                Some(_) => Some(()),
                None => None
            }
        })
    }
}

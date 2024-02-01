use serde::Serialize;
use actix_web::web::Data;
use crate::database::Database;
use crate::api_services::models::ApiService;
use async_trait::async_trait;

#[derive(Serialize)]
struct ServiceQueryParams {
    pub api_name: String,
    pub version: String
}

#[async_trait]
pub trait ApiServiceRepository {
    async fn get_all_services(repo: &Data<Database>) -> Option<Vec<ApiService>>;
    async fn get_service_by_name_and_version(repo: &Data<Database>, api_name: String, version: String) -> Option<ApiService>;
    async fn add_service(repo: &Data<Database>, new_pizza: ApiService) -> Option<ApiService>;
    async fn update_service(repo: &Data<Database>, service_name: String, updated_service: ApiService) -> Option<ApiService>;
    async fn delete_service(repo: &Data<Database>, service_name: String) -> Option<()>;
}

#[async_trait]
impl ApiServiceRepository for Database {

    async fn get_all_services(repo: &Data<Database>) -> Option<Vec<ApiService>> {
        let db_results = repo.db.select("service").await;
        match db_results {
            Ok(all_services) => Some(all_services),
            Err(_) => None
        }
    }

    async fn get_service_by_name_and_version(repo: &Data<Database>, api_name: String, version: String) -> Option<ApiService>{
        let result = repo.db.query(concat!(
            "SELECT * FROM service ",
            "WHERE active = TRUE AND ",
            "api_name = $api_name AND ",
            "version = $version LIMIT 1",
        )).bind(ServiceQueryParams {
            api_name,
            version
        }).await;

        match result {
            Ok(mut response) => {
                let res_inner = response.take(0);
                match res_inner {
                    Ok(record) => record,
                    Err(_) => None
                }
            },
            Err(_) => None
        }
    }   

    async fn add_service(repo: &Data<Database>, new_service: ApiService) -> Option<ApiService> {
        let create_result: Result<Vec<ApiService>, _> = repo.db.create("service").content(new_service).await;
        match create_result{
            Ok(records) => Some(records.get(0).unwrap().clone()),
            Err(_) => None
        }
    }

    
    async fn update_service(_db: &Data<Database>, _service_name: String, _updated_service: ApiService) -> Option<ApiService> {
        None
    }
    async fn delete_service(_db: &Data<Database>, _service_name: String) -> Option<()> {
        None
    }
}
use std::collections::BTreeMap;

use serde;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::io;
use surrealdb::engine::local::{Db, SpeeDb};
use surrealdb::sql::{Strand, Thing, Value};
use surrealdb::{self, opt};

use crate::errors::GatewayError;

pub const USER_TABLE: &str = "gateway_user";
pub const PASSWORD_RESET_TABLE: &str = "password_reset_request";
pub const API_SERVICE_TABLE: &str = "service";
pub const API_ROLE_TABLE: &str = "role";
pub const ROLE_MEMBER_TABLE: &str = "memberOf";
pub const AUTHORIZATIONS_TABLE: &str = "authorizes";
pub const NAMESPACE_MEMBER_ROLE: &str = "__ROLE_NAMESPACE_MEMBER__";
pub const ROLE_NAMESPACE_DELIMITER: &str = "::";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Relationship {
    pub id: Thing,

    #[serde(rename = "in")]
    pub _in: Thing,

    pub out: Thing,
}

#[derive(Clone)]
pub struct Database {
    pub db: surrealdb::Surreal<Db>,
    pub namespace: String,
    pub name: String,
}

impl Database {
    pub async fn init(connection: &str, namespace: &str, name: &str) -> surrealdb::Result<Self> {
        let db = surrealdb::Surreal::new::<SpeeDb>(connection).await?;
        db.use_ns(namespace).use_db(name).await?;

        Ok(Database {
            db,
            namespace: String::from(namespace),
            name: String::from(name),
        })
    }

    // pub async fn get_record<T>(self: &Self, record_id: &Thing) -> Result<T, GatewayError>
    // where T: DeserializeOwned
    // {
    //     self.db
    //     .select(record_id)
    //     .await
    //     .map_err(GatewayError::from)?
    //     .ok_or(GatewayError::NotFound(record_id.tb.clone(), format!("{}", &record_id.id)))
    // }

    pub async fn define_index(
        self: &Self,
        table: &str,
        index_name: &str,
        columns: Vec<&str>,
        options: Option<&str>,
    ) -> io::Result<()> {
        log::debug!("Querying table info for [{}]", table);
        let mut response = self
            .db
            .query(format!("INFO FOR TABLE {}", table))
            .await
            .map_err(|e| io::Error::other(e))?;
        // Grotesque generic data structure for Table info...
        let results: Vec<BTreeMap<String, BTreeMap<String, String>>> =
            response.take(0).map_err(|e| io::Error::other(e))?;
        if let Some(events) = results
            .get(0)
            .ok_or(io::Error::other(format!(
                "No INFO result for table [{}]",
                table
            )))?
            .get(&String::from("indexes"))
        {
            log::debug!("Looking for index [{}] on table [{}]", index_name, table);
            let index_definition = format!(
                "DEFINE INDEX {} ON {} FIELDS {} {}",
                index_name,
                table,
                columns.join(", "),
                options.or(Some("")).unwrap()
            );
            if let Some(existing_index) = events.get(&String::from(index_name)) {
                if existing_index.eq(&index_definition) {
                    log::info!(
                        "Index [{}] already present on table [{}]",
                        index_name,
                        table
                    );
                    return Ok(());
                }
                log::warn!("Overwriting index [{}] on table [{}]", index_name, table);
                log::warn!("Previous Index def: \n{}", existing_index);
                log::warn!("New Index: \n{}", index_definition);
            } else {
                log::warn!("Creating index [{}] on table [{}]", index_name, table);
            }
            self.db
                .query(index_definition)
                .await
                .map_err(|e| io::Error::other(e))?;
        }

        Ok(())
    }

    pub async fn ensure_event_present(
        self: &Self,
        table: &str,
        event_name: &str,
        event_condition: &str,
        event_action: &str,
    ) -> std::io::Result<()> {
        log::debug!("Querying table info for [{}]", table);
        let mut response = self
            .db
            .query(format!("INFO FOR TABLE {}", table))
            .await
            .map_err(|e| io::Error::other(e))?;

        // Grotesque generic data structure for Table info...
        let results: Vec<BTreeMap<String, BTreeMap<String, String>>> =
            response.take(0).map_err(|e| io::Error::other(e))?;

        if let Some(events) = results
            .get(0)
            .ok_or(io::Error::other(format!(
                "No INFO result for table [{}]",
                table
            )))?
            .get(&String::from("events"))
        {
            log::debug!("Looking for event [{}] on table [{}]", event_name, table);
            let event_definition = format!(
                "DEFINE EVENT {} ON {} WHEN {} THEN ({})",
                event_name, table, event_condition, event_action
            );
            if let Some(existing_event) = events.get(&String::from(event_name)) {
                if existing_event.eq(&event_definition) {
                    log::info!(
                        "Event [{}] already present on table [{}]",
                        event_name,
                        table
                    );
                    return Ok(());
                }
                log::warn!("Previous Event def: \n{}", existing_event);
                log::warn!("New Event: \n{}", event_definition);
            }
            log::warn!("Creating event [{}] on table [{}]", event_name, table);
            self.db
                .query(event_definition)
                .await
                .map_err(|e| io::Error::other(e))?;
        } else {
            return Err(io::Error::other(format!(
                "Unable to fetch events for table {}",
                table
            )));
        }

        Ok(())
    }

    pub async fn automate_created_date(self: &Self, table: &str) -> io::Result<()> {
        self.ensure_event_present(
            table,
            "record_create",
            "$event = 'CREATE'",
            format!("UPDATE {} SET created_date = time::now(), last_modified_date = time::now() WHERE id = $after.id", table).as_str(),
        ).await
    }

    pub async fn automate_last_modified_date(self: &Self, table: &str) -> io::Result<()> {
        self.ensure_event_present(
            table,
            "record_update",
            "$event = 'UPDATE' AND $after.last_modified_date == NONE OR ($before.last_modified_date == $after.last_modified_date)",
            format!("UPDATE {} SET last_modified_date = time::now() WHERE id = $after.id", table).as_str(),
        )
        .await
    }

    pub async fn relate(
        self: &Self,
        from: &Thing,
        to: &Thing,
        relationship_table: impl std::fmt::Display,
        content: Option<BTreeMap<String, Value>>,
    ) -> Result<Relationship, GatewayError> {
        let mut query: String = format!("RELATE $from->{}->$to", relationship_table);
        let mut params: BTreeMap<String, Value> = [
            ("from".into(), Value::Thing(from.clone())),
            ("to".into(), Value::Thing(to.clone())),
        ]
        .into();
        if let Some(data) = content {
            query = format!("{} CONTENT $data", query);
            params.insert("data".into(), Value::Object(data.into()));
        }
        let query_result: Option<Relationship> = self.query_record(query, Some(params)).await?;
        query_result.ok_or(GatewayError::DatabaseError(
            "Could not create relationship".to_string(),
        ))
    }

    pub async fn unrelate(
        self: &Self,
        from: &Thing,
        to: &Thing,
        rel: &String,
    ) -> Result<(), GatewayError> {
        let query = "DELETE type::table($rel) WHERE in=$from and out=$to";
        let params: BTreeMap<String, Value> = [
            ("from".into(), Value::Thing(from.clone())),
            ("to".into(), Value::Thing(to.clone())),
            ("rel".into(), Value::Strand(Strand::from(rel.clone()))),
        ]
        .into();
        self.db
            .query(query)
            .bind(params)
            .await
            .map_err(Into::<GatewayError>::into)?;
        Ok(())
    }

    pub async fn query_record<T>(
        self: &Self,
        query: impl opt::IntoQuery + std::fmt::Display,
        params: Option<impl Sized + Serialize>,
    ) -> Result<Option<T>, GatewayError>
    where
        T: DeserializeOwned + std::fmt::Debug,
    {
        let mut db_query = self.db.query(query);
        if let Some(bind_params) = params {
            db_query = db_query.bind(bind_params);
        }
        let mut response = db_query.await.map_err(Into::<GatewayError>::into)?;
        let query_result: Result<Option<T>, surrealdb::Error> = response.take(0);
        let records = query_result.map_err(Into::<GatewayError>::into)?;
        Ok(records)
    }

    pub async fn query_list<T>(
        self: &Self,
        query: impl opt::IntoQuery + std::fmt::Display,
        params: Option<impl Sized + Serialize>,
    ) -> Result<Vec<T>, GatewayError>
    where
        T: DeserializeOwned + std::fmt::Debug,
    {
        let mut db_query = self.db.query(query);
        if let Some(bind_params) = params {
            db_query = db_query.bind(bind_params);
        }
        let mut response = db_query.await.map_err(Into::<GatewayError>::into)?;
        log::debug!("Deserializing object");
        let query_result: Result<Vec<T>, surrealdb::Error> = response.take(0);
        let records = query_result.map_err(Into::<GatewayError>::into)?;
        Ok(records)
    }
}

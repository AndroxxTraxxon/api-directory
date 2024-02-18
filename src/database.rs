use std::collections::BTreeMap;

use surrealdb::engine::local::{Db, SpeeDb};
use surrealdb;
use std::io;

pub const USER_TABLE: &str = "gateway_user";
pub const PASSWORD_RESET_TABLE: &str = "password_reset_request";
pub const API_SERVICE_TABLE: &str = "service";


#[derive(Clone)]
pub struct Database {
    pub db: surrealdb::Surreal<Db>,
    pub namespace: String,
    pub name: String,
}

impl Database {
    pub async fn init(connection: &str, namespace: &str, name: &str) -> surrealdb::Result<Self>{
        let db = surrealdb::Surreal::new::<SpeeDb>(connection).await?;
        db.use_ns(namespace).use_db(name).await?;

        Ok(Database {
            db,
            namespace: String::from(namespace),
            name: String::from(name),
        })
    }

    pub async fn define_index(
        self: &Self,
        table: &str,
        index_name: &str,
        columns: Vec<&str>,
        options: Option<&str>,
    ) -> io::Result<()>{
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
            .ok_or(io::Error::other(format!("No INFO result for table [{}]", table)))?
            .get(&String::from("indexes"))
        {
            log::debug!("Looking for index [{}] on table [{}]", index_name, table);
            let index_definition = format!(
                "DEFINE INDEX {} ON {} FIELDS {} {}",
                index_name, table, columns.join(", "), options.or(Some("")).unwrap()
            );
            if let Some(existing_index) = events.get(&String::from(index_name)) {
                if existing_index.eq(&index_definition) {
                    log::info!("Index [{}] already present on table [{}]", index_name, table);
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
            .ok_or(io::Error::other(format!("No INFO result for table [{}]", table)))?
            .get(&String::from("events"))
        {
            log::debug!("Looking for event [{}] on table [{}]", event_name, table);
            let event_definition = format!(
                "DEFINE EVENT {} ON {} WHEN {} THEN ({})",
                event_name, table, event_condition, event_action
            );
            if let Some(existing_event) = events.get(&String::from(event_name)) {
                if existing_event.eq(&event_definition) {
                    log::info!("Event [{}] already present on table [{}]", event_name, table);
                    return Ok(())
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
            return Err(io::Error::other(format!("Unable to fetch events for table {}", table)));
        }

        Ok(())
    }

    pub async fn automate_created_date(
        self: &Self,
        table: &str
    ) -> io::Result<()> {
        self.ensure_event_present(
            table,
            "record_create",
            "$event = 'CREATE'",
            format!("UPDATE {} SET created_date = time::now(), last_modified_date = time::now() WHERE id = $after.id", table).as_str(),
        ).await
    }

    pub async fn automate_last_modified_date(
        self: &Self,
        table: &str
    )  -> io::Result<()> {
        self.ensure_event_present(
            table,
            "record_update",
            "$event = 'UPDATE' AND $after.last_modified_date == NONE OR ($before.last_modified_date == $after.last_modified_date)",
            format!("UPDATE {} SET last_modified_date = time::now() WHERE id = $after.id", table).as_str(),
        )
        .await
    }
}

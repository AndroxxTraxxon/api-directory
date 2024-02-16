use std::collections::BTreeMap;

use surrealdb::engine::local::{Db, SpeeDb};
use surrealdb::{Error, Surreal};

#[derive(Clone)]
pub struct Database {
    pub db: Surreal<Db>,
    pub namespace: String,
    pub name: String,
}

impl Database {
    pub async fn init(connection: &str, namespace: &str, name: &str) -> Result<Self, Error> {
        let db = Surreal::new::<SpeeDb>(connection).await?;
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
    ) -> Result<(), String> {
        log::debug!("Querying table info for [{}]", table);
        let mut response = self
            .db
            .query(format!("INFO FOR TABLE {}", table))
            .await
            .map_err(|e| e.to_string())?;
        let results: Vec<BTreeMap<String, BTreeMap<String, String>>> =
            response.take(0).map_err(|e| e.to_string())?;
        if let Some(events) = results
            .get(0)
            .ok_or(format!("No INFO result for table [{}]", table))?
            .get(&String::from("indexes"))
        {
            log::debug!("Looking for index [{}] on table [{}]", index_name, table);
            let index_definition = format!(
                "DEFINE INDEX {} ON TABLE  {} FIELDS {} {}",
                index_name, table, columns.join(", "), options.or(Some("")).unwrap()
            );
            if let Some(existing_index) = events.get(&String::from(index_name)) {
                if existing_index == &index_definition {
                    log::info!("Index [{}] already present on table [{}]", index_name, table);
                    log::debug!("{}", existing_index);
                    return Ok(());
                }
                log::warn!("Overwriting index [{}] on table [{}]", index_name, table)
            } else {
                log::warn!("Creating index [{}] on table [{}]", index_name, table);
            }
            self.db
                .query(index_definition)
                .await
                .map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    pub async fn ensure_event_present(
        self: &Self,
        table: &str,
        event_name: &str,
        event_condition: &str,
        event_action: &str,
    ) -> Result<(), String> {
        log::debug!("Querying table info for [{}]", table);
        let mut response = self
            .db
            .query(format!("INFO FOR TABLE {}", table))
            .await
            .map_err(|e| e.to_string())?;
        let results: Vec<BTreeMap<String, BTreeMap<String, String>>> =
            response.take(0).map_err(|e| e.to_string())?;
        if let Some(events) = results
            .get(0)
            .ok_or(format!("No INFO result for table [{}]", table))?
            .get(&String::from("events"))
        {
            log::debug!("Looking for event [{}] on table [{}]", event_name, table);
            let event_definition = format!(
                "DEFINE EVENT {} ON TABLE {} WHEN {} THEN ({})",
                event_name, table, event_condition, event_action
            );
            if let Some(existing_event) = events.get(&String::from(event_name)) {
                if existing_event == &event_definition {
                    log::info!("Event [{}] already present on table [{}]", event_name, table);
                    log::debug!("{}", existing_event);
                    return Ok(())
                }
            }
            log::warn!("Creating event [{}] on table [{}]", event_name, table);
            self.db
                .query(event_definition)
                .await
                .map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    pub async fn automate_created_date(
        self: &Self,
        table: &str
    ) -> Result<(), String> {
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
    )  -> Result<(), String> {
        self.ensure_event_present(
            table,
            "record_update",
            "$event = 'UPDATE' AND $after.last_modified_date == NONE OR ($before.last_modified_date == $after.last_modified_date)",
            format!("UPDATE {} SET last_modified_date = time::now() WHERE id = $after.id", table).as_str(),
        )
        .await
    }
}

use surrealdb::engine::local::{Db, SpeeDb};
use surrealdb::{Surreal, Error};

#[derive(Clone)]
pub struct Database {
    pub db: Surreal<Db>,
    pub namespace: String,
    pub name: String
}

impl Database {
    pub async fn init(connection: &str, namespace: &str, name: &str) -> Result<Self, Error> {

        let db = Surreal::new::<SpeeDb>(connection).await?;
        db.use_ns(namespace).use_db(name).await?;


        
        Ok(Database{
            db,
            namespace: String::from(namespace),
            name: String::from(name)
        })
    }
}
use crate::config::Config as TabssConfig;
use anyhow::Result;
use gluesql::{
    core::ast_builder::{generate_uuid, table, text, Execute},
    prelude::Glue,
    sled_storage::SledStorage,
};
use std::fmt::Debug;

pub struct Database {
    pub storage: Glue<SledStorage>,
}

impl Debug for Database {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Database {}")
    }
}

impl Database {
    pub async fn init(config: &TabssConfig) -> Result<Self> {
        let path = config.db_path();
        let storage =
            SledStorage::new(path.to_str().expect("could not serialize config path")).unwrap();
        let mut storage = Glue::new(storage);

        let db_schema = String::from_utf8_lossy(include_bytes!("schema.sql"));
        let _ = storage.execute(db_schema)?;
        // let _ = table("feeds")
        //     .insert()
        //     .columns("id, title, url")
        //     .values(vec![vec![generate_uuid(), text("Wired"), text("sad")]])
        //     .execute(&mut storage)
        //     .await?;

        Ok(Self { storage })
    }
}

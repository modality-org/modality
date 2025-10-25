use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use async_trait::async_trait;
use anyhow::{Result, Context, anyhow};

use crate::network_datastore::NetworkDatastore;

#[async_trait]
pub trait Model: Sized + Serialize + for<'de> Deserialize<'de> {
    const ID_PATH: &'static str;
    const FIELDS: &'static [&'static str];
    const FIELD_DEFAULTS: &'static [(&'static str, serde_json::Value)];

    fn create_from_json(obj: serde_json::Value) -> Result<Self> {
        let mut model: Self = serde_json::from_value(obj.clone())
            .context("Failed to deserialize object")?;
        for &field in Self::FIELDS {
            if !obj.get(field).is_some() {
                if let Some(default_value) = Self::FIELD_DEFAULTS.iter().find(|&&(k, _)| k == field) {
                    let value = serde_json::to_value(default_value.1.clone())
                        .context("Failed to convert default value to JSON")?;
                    serde_json::from_value(value)
                        .map(|v| model.set_field(field, v))
                        .context("Failed to set default field value")?;
                }
            }
        }
        Ok(model)
    }

    fn set_field(&mut self, field: &str, value: serde_json::Value);

    fn from_json_string(json: &str) -> Result<Self> {
        let obj: serde_json::Value = serde_json::from_str(json)
            .context("Failed to parse JSON string")?;
        Self::create_from_json(obj)
    }

    fn from_json_object(obj: serde_json::Value) -> Result<Self> {
        Self::create_from_json(obj)
    }

    fn to_json_string(&self) -> Result<String> {
        serde_json::to_string(self).context("Failed to serialize to JSON string")
    }

    fn to_json_object(&self) -> Result<serde_json::Value> {
        serde_json::to_value(self).context("Failed to serialize to JSON value")
    }

    async fn save(&self, datastore: &NetworkDatastore) -> Result<()> {
        let json = self.to_json_string()?;
        datastore.put(&self.get_id(), json.as_bytes()).await
            .context("Failed to save model to datastore")
    }

    fn get_id_for(keys: &HashMap<String, String>) -> String {
        let mut id = String::from(Self::ID_PATH);
        for (key, value) in keys {
            id = id.replace(&format!("${{{}}}", key), value);
        }
        id
    }

    fn get_key_names() -> Vec<String> {
        let re = regex::Regex::new(r"\$\{(\w+)\}").unwrap();
        re.captures_iter(Self::ID_PATH)
            .map(|cap| cap[1].to_string())
            .collect()
    }

    fn get_id_keys(&self) -> HashMap<String, String>;

    fn get_id(&self) -> String {
        let keys = self.get_id_keys();
        Self::get_id_for(&keys)
    }

    async fn find_one(datastore: &NetworkDatastore, keys: HashMap<String, String>) -> Result<Option<Self>> {
        let key = Self::get_id_for(&keys);
        match datastore.get_string(&key).await? {
            Some(value) => Ok(Some(Self::from_json_string(&value)?)),
            None => Ok(None),
        }
    }

    async fn reload(&mut self, datastore: &NetworkDatastore) -> Result<()> {
        let keys = self.get_id_keys();
        if let Some(obj) = Self::find_one(datastore, keys).await? {
            *self = obj;
            Ok(())
        } else {
            Err(anyhow!("Key not found: {}", self.get_id()))
        }
    }

    async fn delete(&self, datastore: &NetworkDatastore) -> Result<()> {
        datastore.delete(&self.get_id()).await
            .context("Failed to delete model from datastore")
    }
}

// #[async_trait]
// pub trait ModelExt: Model {
//     fn create_from_json(obj: Value) -> Result<Self> {
//         <Self as Model>::create_from_json(obj)
//     }

//     async fn save(&self, datastore: &NetworkDatastore) -> Result<()> {
//         <Self as Model>::save(self, datastore).await
//     }
//     // Add other methods from Model that you want to expose here
// }

// // This blanket implementation makes ModelExt available for all types that implement Model
// impl<T: Model> ModelExt for T {}
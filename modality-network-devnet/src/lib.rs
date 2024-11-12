use anyhow::Result;
use serde_json::{self, Value};
use lazy_static::lazy_static;
use std::collections::HashMap;

use modality_utils::keypair::Keypair;

pub const KEYPAIRS_JSON: &str = include_str!("../keypairs.json");

lazy_static! {
    pub static ref KEYPAIRS: HashMap<String, Value> = {
        serde_json::from_str(KEYPAIRS_JSON).expect("Failed to parse static keypairs.json")
    };
}
pub struct Devnet;

impl Devnet {
  pub fn new() -> Self {
      Self
  }

  pub async fn get_keypairs(&self, count: Option<usize>) -> Result<Vec<Keypair>> {
      let count = count.unwrap_or_else(|| KEYPAIRS.len());
      
      if count > KEYPAIRS.len() {
          return Err(anyhow::anyhow!("not enough common IDs"));
      }

      let mut result = Vec::with_capacity(count);
      for keypair in KEYPAIRS.values().take(count) {
          result.push(Keypair::from_json_string(&keypair.to_string())?);
      }
      
      Ok(result)
  }

  pub async fn get_keypair_by_index(&self, index: usize) -> Result<Keypair> {
      if index >= KEYPAIRS.len() {
          return Err(anyhow::anyhow!("not enough common IDs"));
      }

      let keypair = KEYPAIRS.values()
          .nth(index)
          .ok_or_else(|| anyhow::anyhow!("Invalid index"))?;
          
      Keypair::from_json_string(&keypair.to_string())
  }

  pub async fn get_keypairs_dict(&self, count: Option<usize>) -> Result<HashMap<String, Keypair>> {
      let count = count.unwrap_or_else(|| KEYPAIRS.len());
      
      if count > KEYPAIRS.len() {
          return Err(anyhow::anyhow!("not enough common IDs"));
      }

      let mut result = HashMap::with_capacity(count);
      for (id, keypair) in KEYPAIRS.iter().take(count) {
          result.insert(id.clone(), Keypair::from_json_string(&keypair.to_string())?);
      }
      
      Ok(result)
  }

  pub fn get_peerids(&self, count: Option<usize>) -> Result<Vec<String>> {
      let count = count.unwrap_or_else(|| KEYPAIRS.len());
      
      if count > KEYPAIRS.len() {
          return Err(anyhow::anyhow!("not enough common IDs"));
      }

      let mut result = Vec::with_capacity(count);
      for keypair in KEYPAIRS.values().take(count) {
          if let Some(id) = keypair.get("id") {
              if let Some(id_str) = id.as_str() {
                  result.push(id_str.to_string());
              }
          }
      }
      
      Ok(result)
  }

  pub fn peerid_of(&self, index: usize) -> Option<String> {
      KEYPAIRS.values()
          .nth(index)
          .and_then(|kp| kp.get("id"))
          .and_then(|id| id.as_str())
          .map(String::from)
  }

  pub fn index_of(&self, peerid: &str) -> Option<usize> {
      KEYPAIRS.values()
          .position(|kp| kp.get("id")
              .and_then(|id| id.as_str())
              .map_or(false, |id| id == peerid))
  }

  pub async fn get_keypair_for(&self, id: &str) -> Result<Keypair> {
      let keypair = KEYPAIRS.get(id)
          .ok_or_else(|| anyhow::anyhow!("Keypair not found"))?;
          
      Keypair::from_json_string(&keypair.to_string())
  }
}

// Public interface
pub fn new() -> Devnet {
  Devnet::new()
}
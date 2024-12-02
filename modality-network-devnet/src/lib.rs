use anyhow::Result;
use serde_json::{self, Value};
use lazy_static::lazy_static;
use std::collections::HashMap;
use modality_network_datastore::{Model, NetworkDatastore};
use modality_network_datastore::models::{Round, Page};

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

  pub fn get_keypairs_dict(count: usize) -> Result<HashMap<String, Keypair>> {
      if count > KEYPAIRS.len() {
          return Err(anyhow::anyhow!("not enough common IDs"));
      }

      let mut result = HashMap::with_capacity(count);
      for (id, keypair) in KEYPAIRS.iter().take(count) {
          result.insert(id.clone(), Keypair::from_json_string(&keypair.to_string())?);
      }
      
      Ok(result)
  }

  pub fn get_peerids(count: usize) -> Result<Vec<String>> {
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

  pub async fn setup_datastore_scribes(ds: &mut NetworkDatastore, count: usize) -> Result<()> {
    let peers_hashmap = Devnet::get_keypairs_dict(count)?;
    ds.set_current_round(1).await?;
    let mut round = Round::create_from_json(serde_json::json!({"round": 1}))?;
    for (peer_id_str, _peer_id_keypair) in &peers_hashmap {
        round.add_scribe(peer_id_str.to_string());
    }
    round.save(ds).await?;
    Devnet::add_fully_connected_empty_round(ds).await?;
    ds.set_current_round(2).await?;
    Ok(())
  }

    pub async fn add_fully_connected_empty_round(ds: &mut NetworkDatastore) -> Result<()> {
        let round_num = ds.get_current_round().await?;
        let round = Round::find_one(&ds, HashMap::from([("round".into(), "1".into())])).await?.unwrap();
        let peers_hashmap = Devnet::get_keypairs_dict(round.scribes.len())?; 
        for peer_id_str in round.scribes.clone() {
            if round_num > 1 {
                // TODO find last_round_certs
            }
            let mut page = Page::create_from_json(serde_json::json!({
                "scribe": peer_id_str,
                "round": round_num,
                "events": [],
                // "last_round_certs"
            }))?;
            page.generate_sig(&peers_hashmap[&peer_id_str])?;
            page.save(&ds).await?;
            for acking_peer_id_str in round.scribes.clone() {
                let ack = page.generate_ack(&peers_hashmap[&acking_peer_id_str])?;
                page.add_ack(ack)?;
            }
            page.generate_cert(&peers_hashmap[&peer_id_str])?;
            page.save(&ds).await?;
        }
        Ok(())
    }
}

// Public interface
pub fn new() -> Devnet {
  Devnet::new()
}
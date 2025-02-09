use anyhow::{Result};
use std::sync::Arc;
use crate::consensus_math::calculate_2f_plus_1;
use crate::election::Election;
use super::Sequencing;

#[derive(Clone)]
pub struct StaticAuthority {
    #[allow(dead_code)]
    election: Arc<Election>,
    scribes: Arc<Vec<String>>,  // Use Arc for scribes
}

impl StaticAuthority {
    pub async fn create(scribes: Vec<String>, election: Election) -> Self {
        StaticAuthority {
            election: Arc::new(election),
            scribes: Arc::new(scribes),
        }
    }
}

#[async_trait::async_trait]
impl Sequencing for StaticAuthority {
    async fn get_scribes_at_round_id(&self, _round: u64) -> Result<Vec<String>> {
        Ok((*self.scribes).clone())
    }
    
    async fn consensus_threshold_at_round_id(&self, _round: u64) -> Result<u64> {
        Ok(calculate_2f_plus_1(self.scribes.len() as f64))
    }
}

// Remove Send + Sync derive, as Arc handles thread safety
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_static_authority() -> anyhow::Result<()> {
        let scribes = vec!["scribe1".to_string(), "scribe2".to_string(), "scribe3".to_string()];
        let election = Election::RoundRobin(crate::election::round_robin::RoundRobin::create());
        
        let sa = StaticAuthority::create(scribes.clone(), election).await;
        
        let round_scribes = sa.get_scribes_at_round_id(0).await?;
        assert_eq!(round_scribes, scribes);
        
        let threshold = sa.consensus_threshold_at_round_id(0).await?;
        assert_eq!(threshold, 3);  // 2f+1 where n=3
        Ok(())
    }
}
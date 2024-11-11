// use async_trait::async_trait;

use crate::consensus_math::calculate_2f_plus_1;
use crate::election::Election;
pub struct StaticAuthority {
    #[allow(dead_code)]
    election: Election,
    scribes: Vec<String>,  // Assuming scribes are identified by strings - adjust type as needed
}

impl StaticAuthority {
    pub async fn create(scribes: Vec<String>, election: Election) -> Self {
        StaticAuthority {
            election,
            scribes,
        }
    }

    pub async fn get_scribes_at_round(&self, _round: u64) -> Vec<String> {
        self.scribes.clone()
    }

    pub async fn consensus_threshold_for_round(&self, _round: u64) -> i64 {
        calculate_2f_plus_1(self.scribes.len() as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_static_authority() {
        let scribes = vec!["scribe1".to_string(), "scribe2".to_string(), "scribe3".to_string()];
        let election = Election::RoundRobin(crate::election::round_robin::RoundRobin::create());
        
        let sa = StaticAuthority::create(scribes.clone(), election).await;
        
        let round_scribes = sa.get_scribes_at_round(1).await;
        assert_eq!(round_scribes, scribes);
        
        let threshold = sa.consensus_threshold_for_round(1).await;
        assert_eq!(threshold, 3);  // 2f+1 where n=3
    }
}
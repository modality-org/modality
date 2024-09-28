#[cfg(test)]
mod tests {
    use super::*;
    use crate::NetworkDatastore;

    #[tokio::test]
    async fn test_round() -> Result<()> {
        let datastore = NetworkDatastore::create_in_memory()?;
        
        let round = Round::from(serde_json::json!({"round": 1}))?;
        round.save(&datastore).await?;
        
        let round = Round::from(serde_json::json!({"round": 2}))?;
        round.save(&datastore).await?;
        
        let round = Round::from(serde_json::json!({"round": 3}))?;
        round.save(&datastore).await?;
        
        let max_round = Round::find_max_id(&datastore).await?;
        assert_eq!(max_round, Some(3));

        Ok(())
    }

    #[test]
    fn test_add_remove_scribe() -> Result<()> {
        let mut round = Round::from(serde_json::json!({"round": 1}))?;
        
        round.add_scribe("peer1".to_string());
        assert_eq!(round.scribes, vec!["peer1"]);

        round.add_scribe("peer2".to_string());
        assert_eq!(round.scribes, vec!["peer1", "peer2"]);

        round.remove_scribe("peer1");
        assert_eq!(round.scribes, vec!["peer2"]);

        Ok(())
    }
}
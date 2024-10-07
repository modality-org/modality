#[cfg(test)]
mod tests {
    use super::*;
    use crate::NetworkDatastore;

    #[tokio::test]
    async fn test_round_message() -> Result<()> {
        let datastore = NetworkDatastore::create_in_memory()?;
        
        // Create and save some test messages
        let messages = vec![
            RoundMessage {
                round: 1,
                scribe: "scribe1".to_string(),
                r#type: "type1".to_string(),
                seen_at_round: Some(1),
                content: serde_json::json!({"key": "value1"}),
            },
            RoundMessage {
                round: 1,
                scribe: "scribe2".to_string(),
                r#type: "type1".to_string(),
                seen_at_round: Some(1),
                content: serde_json::json!({"key": "value2"}),
            },
            RoundMessage {
                round: 1,
                scribe: "scribe3".to_string(),
                r#type: "type2".to_string(),
                seen_at_round: Some(1),
                content: serde_json::json!({"key": "value3"}),
            },
        ];

        for msg in &messages {
            msg.save(&datastore).await?;
        }

        // Test find_all_in_round_of_type
        let found_messages = RoundMessage::find_all_in_round_of_type(&datastore, 1, "type1").await?;
        assert_eq!(found_messages.len(), 2);
        assert!(found_messages.iter().any(|m| m.scribe == "scribe1"));
        assert!(found_messages.iter().any(|m| m.scribe == "scribe2"));

        let found_messages = RoundMessage::find_all_in_round_of_type(&datastore, 1, "type2").await?;
        assert_eq!(found_messages.len(), 1);
        assert_eq!(found_messages[0].scribe, "scribe3");

        // Test non-existent round or type
        let found_messages = RoundMessage::find_all_in_round_of_type(&datastore, 2, "type1").await?;
        assert_eq!(found_messages.len(), 0);

        let found_messages = RoundMessage::find_all_in_round_of_type(&datastore, 1, "type3").await?;
        assert_eq!(found_messages.len(), 0);

        Ok(())
    }

    #[test]
    fn test_get_id_keys() {
        let message = RoundMessage {
            round: 1,
            scribe: "scribe1".to_string(),
            r#type: "type1".to_string(),
            seen_at_round: Some(1),
            content: serde_json::json!({"key": "value"}),
        };

        let keys = message.get_id_keys();
        assert_eq!(keys.get("round"), Some(&"1".to_string()));
        assert_eq!(keys.get("scribe"), Some(&"scribe1".to_string()));
        assert_eq!(keys.get("type"), Some(&"type1".to_string()));
    }
}
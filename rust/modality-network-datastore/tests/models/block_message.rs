#[cfg(test)]
mod tests {
    use anyhow::{Result};
    use modality_network_datastore::NetworkDatastore;
    use modality_network_datastore::models::block::prelude::*;
    use modality_network_datastore::models::block_message::BlockMessage;

    #[tokio::test]
    async fn test_block_message() -> Result<()> {
        let datastore = NetworkDatastore::create_in_memory()?;
        
        // Create and save some test messages
        let messages = vec![
            BlockMessage {
                round_id: 1,
                peer_id: "scribe1".to_string(),
                r#type: "type1".to_string(),
                seen_at_block_id: Some(1),
                content: serde_json::json!({"key": "value1"}),
            },
            BlockMessage {
                round_id: 1,
                peer_id: "scribe2".to_string(),
                r#type: "type1".to_string(),
                seen_at_block_id: Some(1),
                content: serde_json::json!({"key": "value2"}),
            },
            BlockMessage {
                round_id: 1,
                peer_id: "scribe3".to_string(),
                r#type: "type2".to_string(),
                seen_at_block_id: Some(1),
                content: serde_json::json!({"key": "value3"}),
            },
        ];

        for msg in &messages {
            msg.save(&datastore).await?;
        }

        // Test find_all_in_block_of_type
        let found_messages = BlockMessage::find_all_in_block_of_type(&datastore, 1, "type1").await?;
        assert_eq!(found_messages.len(), 2);
        assert!(found_messages.iter().any(|m| m.peer_id == "scribe1"));
        assert!(found_messages.iter().any(|m| m.peer_id == "scribe2"));

        let found_messages = BlockMessage::find_all_in_block_of_type(&datastore, 1, "type2").await?;
        assert_eq!(found_messages.len(), 1);
        assert_eq!(found_messages[0].peer_id, "scribe3");

        // Test non-existent block or type
        let found_messages = BlockMessage::find_all_in_block_of_type(&datastore, 2, "type1").await?;
        assert_eq!(found_messages.len(), 0);

        let found_messages = BlockMessage::find_all_in_block_of_type(&datastore, 1, "type3").await?;
        assert_eq!(found_messages.len(), 0);

        Ok(())
    }

    #[test]
    fn test_get_id_keys() {
        let message = BlockMessage {
            round_id: 1,
            peer_id: "scribe1".to_string(),
            r#type: "type1".to_string(),
            seen_at_block_id: Some(1),
            content: serde_json::json!({"key": "value"}),
        };

        let keys = message.get_id_keys();
        assert_eq!(keys.get("round_id"), Some(&"1".to_string()));
        assert_eq!(keys.get("peer_id"), Some(&"scribe1".to_string()));
        assert_eq!(keys.get("type"), Some(&"type1".to_string()));
    }
}
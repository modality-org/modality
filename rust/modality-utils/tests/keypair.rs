use anyhow::Result;
use modality_utils::keypair::Keypair;
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_generate_keypair() {
        let result = Keypair::generate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_public_key_as_base58_identity() {
        let keypair = Keypair::generate().unwrap();
        let public_key_id = keypair.public_key_as_base58_identity();
        assert!(!public_key_id.is_empty());
        assert_eq!(public_key_id.len(), 52);
    }

    #[test]
    fn test_public_key_to_multiaddr_string() {
        let keypair = Keypair::generate().unwrap();
        let multiaddr = keypair.public_key_to_multiaddr_string();
        assert!(multiaddr.starts_with("/ed25519-pub/"));
        println!("Multiaddr: {}", multiaddr);
        assert_eq!(multiaddr.len(), 13 + 52);
    }

    #[test]
    fn test_as_public_json() -> Result<()> {
        let keypair = Keypair::generate()?;
        let json = keypair.as_public_json()?;
        assert_eq!(json.id, keypair.public_key_as_base58_identity());
        assert!(!json.public_key.is_empty());
        assert!(json.private_key.is_none());
        Ok(())
    }

    #[test]
    fn test_as_json() -> Result<()> {
        let keypair = Keypair::generate()?;
        let json = keypair.as_json()?;
        assert_eq!(json.id, keypair.public_key_as_base58_identity());
        assert!(!json.public_key.is_empty());
        assert!(json.private_key.is_some());
        Ok(())
    }

    #[test]
    fn test_sign_and_verify_string() -> Result<()> {
        let keypair = Keypair::generate()?;
        let message = "Hello, world!";
        let signature = keypair.sign_string_as_base64_pad(message)?;
        
        let verification_result = keypair.verify_signature_for_string(&signature, message)?;
        assert!(verification_result);

        let wrong_message = "Hello, World!";
        let wrong_verification_result = keypair.verify_signature_for_string(&signature, wrong_message)?;
        assert!(!wrong_verification_result);

        Ok(())
    }

    #[test]
    fn test_sign_and_verify_json() -> Result<()> {
        let keypair = Keypair::generate()?;
        let json_data = json!({
            "name": "Alice",
            "age": 30
        });

        let signature = keypair.sign_json(&json_data)?;
        
        let verification_result = keypair.verify_json(&signature, &json_data)?;
        assert!(verification_result);

        let wrong_json_data = json!({
            "name": "Alice",
            "age": 31
        });
        let wrong_verification_result = keypair.verify_json(&signature, &wrong_json_data)?;
        assert!(!wrong_verification_result);

        Ok(())
    }

    #[test]
    fn test_from_and_to_json_string() -> Result<()> {
        let original_keypair = Keypair::generate()?;
        let json_string = original_keypair.as_json_string()?;
        
        let recovered_keypair = Keypair::from_json_string(&json_string)?;
        
        assert_eq!(
            original_keypair.public_key_as_base58_identity(),
            recovered_keypair.public_key_as_base58_identity()
        );
        
        // Verify that the recovered keypair can sign and verify correctly
        let message = "Test message";
        let signature = recovered_keypair.sign_string_as_base64_pad(message)?;
        assert!(recovered_keypair.verify_signature_for_string(&signature, message)?);

        Ok(())
    }

    #[test]
    fn test_sign_json_element() -> Result<()> {
        let keypair = Keypair::generate()?;
        let mut json_data = json!({
            "payload": {
                "name": "Alice",
                "age": 30
            }
        });

        keypair.sign_json_element(&mut json_data, "payload", ".signature")?;

        assert!(json_data["payload.signature"].is_string());
        let signature = json_data["payload.signature"].as_str().unwrap();
        let verification_result = keypair.verify_json(signature, &json_data["payload"])?;
        assert!(verification_result);

        Ok(())
    }

    #[test]
    fn test_sign_json_as_key() -> Result<()> {
        let keypair = Keypair::generate()?;
        let mut json_data = json!({
            "name": "Alice",
            "age": 30
        });

        keypair.sign_json_as_key(&mut json_data, "signature")?;

        assert!(json_data["signature"].is_string());
        let verification_result = keypair.verify_json_with_signature_key(&json_data, "signature")?;
        assert!(verification_result);

        Ok(())
    }

    #[test]
    fn test_from_public_multiaddress() -> Result<()> {
        let multiaddr = "/ed25519-pub/12D3KooW9pypLnRn67EFjiWgEiDdqo8YizaPn8yKe5cNJd3PGnMB";
        let keypair = Keypair::from_public_multiaddress(multiaddr)?;
        
        assert_eq!(keypair.public_key_to_multiaddr_string(), multiaddr);

        Ok(())
    }
}
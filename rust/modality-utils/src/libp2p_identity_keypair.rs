use anyhow::{Result};
use libp2p::identity::Keypair;
use base64::prelude::*;
use zeroize::Zeroizing;

pub async fn libp2p_identity_from_private_key(private_key: &str) -> Result<libp2p::identity::Keypair> {
  let private_key_bytes = BASE64_STANDARD.decode(private_key)?;
  let keypair = Keypair::from_protobuf_encoding(&Zeroizing::new(private_key_bytes))?; 
  Ok(keypair)
}

mod tests {
  #[allow(unused_imports)]
  use super::*;

  #[tokio::test]
  async fn test_libp2p_identity_from_private_key() -> Result<()> {
      // Generate a test keypair and encode it
      let original_keypair = Keypair::generate_ed25519();
      let encoded_private_key = BASE64_STANDARD.encode(original_keypair.to_protobuf_encoding()?);

      // Test the function
      let decoded_keypair = libp2p_identity_from_private_key(&encoded_private_key).await?;

      // Verify the public keys match
      assert_eq!(
          original_keypair.public().to_peer_id(),
          decoded_keypair.public().to_peer_id(),
          "Public keys should match"
      );

      Ok(())
  }

  #[tokio::test]
  async fn test_invalid_private_key() {
      // Test with invalid base64
      let result = libp2p_identity_from_private_key("invalid-base64!@#").await;
      assert!(result.is_err(), "Should fail with invalid base64");

      // Test with valid base64 but invalid key data
      let result = libp2p_identity_from_private_key("aGVsbG8=").await; // "hello" in base64
      assert!(result.is_err(), "Should fail with invalid key data");
  }
}
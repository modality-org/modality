use anyhow::{Result};
use libp2p::identity;
use libp2p::identity::Keypair;
use base64::prelude::*;
use zeroize::Zeroizing;

pub async fn libp2p_identity_from_private_key(private_key: &str) -> Result<identity::Keypair> {
  let private_key_bytes = BASE64_STANDARD.decode(private_key)?;
  let keypair = Keypair::from_protobuf_encoding(&Zeroizing::new(private_key_bytes))?; 
  Ok(keypair)
}
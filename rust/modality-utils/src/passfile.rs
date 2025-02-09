use anyhow::Result;
use std::path::PathBuf;

use crate::keypair::Keypair;
// use rpassword::read_password;

#[derive(Clone)]
pub struct Passfile {
  pub filepath: PathBuf,
  pub keypair: Keypair,
}

impl Passfile {
  pub async fn load_file(filepath: PathBuf, _interactive: bool) -> Result<Self> {
    // TODO if interactive ask for password if encrypted
    let keypair = Keypair::from_json_file(filepath.to_str().unwrap())?;
    Ok(Self { filepath, keypair })
  }
}

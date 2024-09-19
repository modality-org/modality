use std::fs;
use std::path::Path;
use serde::{Serialize, Deserialize};
use serde_json;
use libp2p::core::identity::{Keypair as Libp2pKeypair, PublicKey};
use libp2p::core::multiaddr::Protocol;
use base58::{ToBase58, FromBase58};
use ed25519_dalek::{Signer, Verifier, SecretKey, PublicKey as Ed25519PublicKey};
use rand::rngs::OsRng;

#[derive(Clone)]
pub struct Keypair {
    key: Libp2pKeypair,
}

#[derive(Serialize, Deserialize)]
struct KeypairJson {
    id: String,
    public_key: String,
    private_key: Option<String>,
}

impl Keypair {
    pub fn new(key: Libp2pKeypair) -> Self {
        Self { key }
    }

    pub fn generate() -> Self {
        let keypair = Libp2pKeypair::generate_ed25519();
        Self::new(keypair)
    }

    // SSH keys methods would go here, but they require a custom implementation
    // of SSHPem, which is not provided in the original code

    pub fn public_key_as_base58_identity(&self) -> String {
        let public_key_bytes = self.key.public().encode_protobuf();
        let multihash = multihash::Identity::digest(&public_key_bytes);
        multihash.to_base58()
    }

    pub fn as_public_key_id(&self) -> String {
        self.public_key_as_base58_identity()
    }

    pub fn as_public_address(&self) -> String {
        self.public_key_as_base58_identity()
    }

    pub fn public_key_as_base64_pad(&self) -> String {
        base64::encode_config(self.key.public().encode_protobuf(), base64::STANDARD_NO_PAD)
    }

    pub fn private_key_as_base64_pad(&self) -> String {
        base64::encode_config(self.key.encode_protobuf(), base64::STANDARD_NO_PAD)
    }

    pub fn public_key_to_multiaddr_string(&self) -> String {
        let public_key_id = self.public_key_as_base58_identity();
        format!("/ed25519-pub/{}", public_key_id)
    }

    pub fn as_public_multiaddress(&self) -> String {
        self.public_key_to_multiaddr_string()
    }

    pub fn from_public_key(public_key_id: &str, key_type: &str) -> Option<Self> {
        Self::from_public_multiaddress(&format!("/{}-pub/{}", key_type, public_key_id))
    }

    pub fn from_public_multiaddress(multiaddress: &str) -> Option<Self> {
        let parts: Vec<&str> = multiaddress.split('/').collect();
        if parts.len() != 3 || !parts[1].ends_with("-pub") {
            return None;
        }

        let public_key_bytes = parts[2].from_base58().ok()?;
        let public_key = PublicKey::try_decode_protobuf(&public_key_bytes).ok()?;
        Some(Self::new(Libp2pKeypair::from(public_key)))
    }

    pub fn from_json(json: &KeypairJson) -> Option<Self> {
        if let Some(private_key) = &json.private_key {
            let key_bytes = base64::decode_config(private_key, base64::STANDARD_NO_PAD).ok()?;
            let keypair = Libp2pKeypair::try_decode_protobuf(&key_bytes).ok()?;
            Some(Self::new(keypair))
        } else if let Some(public_key) = &json.public_key {
            let key_bytes = base64::decode_config(public_key, base64::STANDARD_NO_PAD).ok()?;
            let public_key = PublicKey::try_decode_protobuf(&key_bytes).ok()?;
            Some(Self::new(Libp2pKeypair::from(public_key)))
        } else {
            None
        }
    }

    pub fn from_json_string(json_str: &str) -> Option<Self> {
        let json: KeypairJson = serde_json::from_str(json_str).ok()?;
        Self::from_json(&json)
    }

    pub fn from_json_file<P: AsRef<Path>>(path: P) -> Option<Self> {
        let json_str = fs::read_to_string(path).ok()?;
        Self::from_json_string(&json_str)
    }

    pub fn as_public_json(&self) -> KeypairJson {
        KeypairJson {
            id: self.public_key_as_base58_identity(),
            public_key: self.public_key_as_base64_pad(),
            private_key: None,
        }
    }

    pub fn as_public_json_string(&self) -> String {
        serde_json::to_string(&self.as_public_json()).unwrap()
    }

    pub fn as_public_json_file<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        let json_string = self.as_public_json_string();
        fs::write(path, json_string)
    }

    pub fn as_json(&self) -> KeypairJson {
        KeypairJson {
            id: self.public_key_as_base58_identity(),
            public_key: self.public_key_as_base64_pad(),
            private_key: Some(self.private_key_as_base64_pad()),
        }
    }

    pub fn as_json_string(&self) -> String {
        serde_json::to_string(&self.as_json()).unwrap()
    }

    pub fn as_json_file<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        let json_string = self.as_json_string();
        fs::write(path, json_string)
    }

    pub fn sign_bytes(&self, bytes: &[u8]) -> Vec<u8> {
        match &self.key {
            Libp2pKeypair::Ed25519(keypair) => {
                let secret_key = SecretKey::from_bytes(&keypair.secret().as_ref()).unwrap();
                let signature = secret_key.sign(bytes);
                signature.to_bytes().to_vec()
            }
            _ => panic!("Only Ed25519 keys are supported for signing"),
        }
    }

    pub fn sign_string(&self, s: &str) -> Vec<u8> {
        self.sign_bytes(s.as_bytes())
    }

    pub fn sign_string_as_base64_pad(&self, s: &str) -> String {
        let signature = self.sign_string(s);
        base64::encode_config(signature, base64::STANDARD_NO_PAD)
    }

    pub fn sign_json<T: Serialize>(&self, json: &T) -> String {
        let json_string = serde_json::to_string(json).unwrap();
        self.sign_string_as_base64_pad(&json_string)
    }

    pub fn sign_json_element<T: Serialize>(&self, json: &mut serde_json::Value, name: &str, suffix: &str) {
        let element = json.get(name).unwrap().clone();
        let signature = self.sign_json(&element);
        json[format!("{}{}", name, suffix)] = serde_json::Value::String(signature);
    }

    pub fn sign_json_as_key<T: Serialize>(&self, json: &mut serde_json::Value, key: &str) {
        let signature = self.sign_json(json);
        json[key] = serde_json::Value::String(signature);
    }

    pub fn verify_signature_for_bytes(&self, signature: &str, bytes: &[u8]) -> bool {
        let signature_bytes = base64::decode_config(signature, base64::STANDARD_NO_PAD).unwrap();
        match &self.key.public() {
            PublicKey::Ed25519(public_key) => {
                let ed_public_key = Ed25519PublicKey::from_bytes(&public_key.encode()).unwrap();
                let signature = ed25519_dalek::Signature::from_bytes(&signature_bytes).unwrap();
                ed_public_key.verify(bytes, &signature).is_ok()
            }
            _ => panic!("Only Ed25519 keys are supported for verification"),
        }
    }

    pub fn verify_signature_for_string(&self, signature: &str, s: &str) -> bool {
        self.verify_signature_for_bytes(signature, s.as_bytes())
    }

    pub fn verify_json<T: Serialize>(&self, signature: &str, json: &T) -> bool {
        let json_string = serde_json::to_string(json).unwrap();
        self.verify_signature_for_string(signature, &json_string)
    }

    pub fn verify_signatures_in_json(&self, json: &serde_json::Value, suffix: &str) -> bool {
        for (key, value) in json.as_object().unwrap() {
            if key.ends_with(suffix) {
                let original_key = key.trim_end_matches(suffix);
                if let Some(original_value) = json.get(original_key) {
                    let signature = value.as_str().unwrap();
                    if !self.verify_json(signature, original_value) {
                        return false;
                    }
                }
            }
        }
        true
    }
}
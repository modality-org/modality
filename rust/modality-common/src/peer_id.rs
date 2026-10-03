//! The text forms of a peer ID (a Modality ID is one).
//!
//! libp2p writes a peer ID either as its bare multihash in base58
//! (`12D3KooW…`, the legacy form every implementation prints) or as a CIDv1
//! with the `libp2p-key` codec in a multibase (`bafz…` in base32, `k51…` in
//! base36). All name the same bytes, and all begin with the same characters
//! for every ed25519 key.
//!
//! The Modality form is the base32 CID written backwards
//! (`imqi74td…aiajaazfab`): lowercase, and its first characters differ from
//! key to key, so a short prefix tells IDs apart. Its tail is fixed. It is the
//! standard Modality ID, and what `.id` values hold; see
//! `docs/concepts/modality-ids.md`.
//!
//! One key has many spellings, so rules never compare IDs as text: both
//! sides go through [`key_form`] first. Commit signatures are still keyed in
//! base58, contract IDs are base58, and a `/p2p/` multiaddr part must be.

use anyhow::{anyhow, bail, Result};
use libp2p_identity::PeerId;
use multibase::Base;

/// CID version 1, as its one-byte varint.
const CID_V1: u8 = 0x01;
/// The `libp2p-key` multicodec, as its one-byte varint.
const LIBP2P_KEY: u8 = 0x72;

/// A peer ID in any text form. Per the libp2p spec, text starting with `1`
/// or `Qm` is a base58 multihash; otherwise it is a CIDv1 whose codec is
/// `libp2p-key`, or the Modality form (a base32 one, backwards).
pub fn parse_peer_id(text: &str) -> Result<PeerId> {
    if text.starts_with('1') || text.starts_with("Qm") {
        return text
            .parse::<PeerId>()
            .map_err(|e| anyhow!("{text} is not a peer ID: {e}"));
    }
    let forwards = parse_cid(text);
    if forwards.is_ok() || !text.ends_with('b') {
        return forwards;
    }
    let backwards: String = text.chars().rev().collect();
    parse_cid(&backwards).map_err(|_| anyhow!("{text} is not a peer ID"))
}

fn parse_cid(text: &str) -> Result<PeerId> {
    let (_, bytes) =
        multibase::decode(text).map_err(|e| anyhow!("{text} is not a peer ID: {e}"))?;
    match bytes.as_slice() {
        [CID_V1, LIBP2P_KEY, multihash @ ..] => PeerId::from_bytes(multihash)
            .map_err(|e| anyhow!("{text} is not a peer ID: {e}")),
        [CID_V1, ..] => bail!("{text} is a CID, but not of a libp2p key"),
        _ => bail!("{text} is not a peer ID"),
    }
}

/// `text` as the Modality form of the key it names, in any spelling: a
/// peer ID in any text form, or a 32-byte ed25519 key in hex. Text naming no
/// key is returned as is. Compare keys through this, never as text.
pub fn key_form(text: &str) -> String {
    if let Ok(peer_id) = parse_peer_id(text) {
        return modality_peer_id(&peer_id);
    }
    if text.len() == 64 {
        if let Some(key) = hex::decode(text)
            .ok()
            .and_then(|bytes| libp2p_identity::ed25519::PublicKey::try_from_bytes(&bytes).ok())
        {
            return modality_peer_id(&libp2p_identity::PublicKey::from(key).to_peer_id());
        }
    }
    text.to_string()
}

/// The Modality form of an ID given in any text form, for writing into a
/// `.id` value; other text as given, for the caller to refuse.
pub fn id_value(text: &str) -> String {
    parse_peer_id(text)
        .map(|peer_id| modality_peer_id(&peer_id))
        .unwrap_or_else(|_| text.to_string())
}

/// The base58 form of a peer ID given in any text form.
pub fn canonical_peer_id(text: &str) -> Result<String> {
    parse_peer_id(text).map(|peer_id| peer_id.to_base58())
}

/// `text` in base58 if it is a peer ID in another form; otherwise `text` as
/// given, for IDs that need not be peer IDs (hub contracts are `c_…`) and
/// for the caller to refuse what is neither.
pub fn normalize_peer_id(text: &str) -> String {
    canonical_peer_id(text).unwrap_or_else(|_| text.to_string())
}

/// [`normalize_peer_id`] as a clap `value_parser`, for ID arguments.
pub fn peer_id_arg(text: &str) -> Result<String, std::convert::Infallible> {
    Ok(normalize_peer_id(text))
}

/// The CIDv1 form of `peer_id`, in base32 as the libp2p spec recommends.
pub fn peer_id_to_cid(peer_id: &PeerId) -> String {
    let mut bytes = vec![CID_V1, LIBP2P_KEY];
    bytes.extend_from_slice(&peer_id.to_bytes());
    multibase::encode(Base::Base32Lower, bytes)
}

/// The Modality form of `peer_id`: its base32 CID, backwards.
pub fn modality_peer_id(peer_id: &PeerId) -> String {
    peer_id_to_cid(peer_id).chars().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // An ed25519 Modality ID and its CIDv1 forms, from the libp2p spec's
    // encoding (computed independently of this module).
    const BASE58: &str = "12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd";
    const BASE32: &str = "bafzaajaiaejcaabcornvmd5g5bolzmcr6smsje7owdd3xk7lojqydmthdt47iqmi";
    const MODALITY: &str = "imqi74tdhtmdyqjol7kx3ddwo7ejsms6rcmzlob5g5dmvnrocbaacjeaiajaazfab";

    #[test]
    fn both_forms_name_the_same_peer() {
        assert_eq!(canonical_peer_id(BASE58).unwrap(), BASE58);
        assert_eq!(canonical_peer_id(BASE32).unwrap(), BASE58);
        assert_eq!(peer_id_to_cid(&parse_peer_id(BASE58).unwrap()), BASE32);
    }

    #[test]
    fn the_modality_form_is_the_base32_cid_backwards() {
        assert_eq!(modality_peer_id(&parse_peer_id(BASE58).unwrap()), MODALITY);
        assert_eq!(canonical_peer_id(MODALITY).unwrap(), BASE58);
        // Only the base32 CID is read backwards.
        let base36 = multibase::encode(
            Base::Base36Lower,
            [&[CID_V1, LIBP2P_KEY][..], &parse_peer_id(BASE58).unwrap().to_bytes()].concat(),
        );
        assert!(parse_peer_id(&base36.chars().rev().collect::<String>()).is_err());
    }

    #[test]
    fn every_spelling_of_a_key_has_one_key_form() {
        let secret = libp2p_identity::ed25519::Keypair::generate();
        let peer_id = libp2p_identity::PublicKey::from(secret.public()).to_peer_id();
        let hex = hex::encode(secret.public().to_bytes());
        let spellings = [
            peer_id.to_base58(),
            peer_id_to_cid(&peer_id),
            modality_peer_id(&peer_id),
            hex.clone(),
            hex.to_uppercase(),
        ];
        for spelling in &spellings {
            assert_eq!(key_form(spelling), modality_peer_id(&peer_id), "{spelling}");
        }
        assert_eq!(key_form("alice_key"), "alice_key");
        assert_eq!(key_form("c_0123456789abcdef"), "c_0123456789abcdef");
    }

    #[test]
    fn id_values_are_written_in_the_modality_form() {
        assert_eq!(id_value(BASE58), MODALITY);
        assert_eq!(id_value(BASE32), MODALITY);
        assert_eq!(id_value(MODALITY), MODALITY);
        assert_eq!(id_value("not an id"), "not an id");
    }

    #[test]
    fn normalizing_rewrites_only_cids() {
        assert_eq!(normalize_peer_id(BASE32), BASE58);
        assert_eq!(normalize_peer_id(MODALITY), BASE58);
        assert_eq!(normalize_peer_id(BASE58), BASE58);
        assert_eq!(normalize_peer_id("c_0123456789abcdef"), "c_0123456789abcdef");
    }

    #[test]
    fn any_multibase_of_the_cid_parses() {
        let peer_id = parse_peer_id(BASE58).unwrap();
        let mut bytes = vec![CID_V1, LIBP2P_KEY];
        bytes.extend_from_slice(&peer_id.to_bytes());
        for base in [Base::Base36Lower, Base::Base32Upper, Base::Base58Btc] {
            let text = multibase::encode(base, &bytes);
            assert_eq!(canonical_peer_id(&text).unwrap(), BASE58, "{text}");
        }
    }

    #[test]
    fn a_fresh_key_round_trips() {
        let peer_id = libp2p_identity::Keypair::generate_ed25519().public().to_peer_id();
        let cid = peer_id_to_cid(&peer_id);
        assert!(cid.starts_with("bafzaa"), "{cid}");
        assert_eq!(parse_peer_id(&cid).unwrap(), peer_id);
        let modality = modality_peer_id(&peer_id);
        assert!(modality.ends_with("aiajaazfab"), "{modality}");
        assert_eq!(parse_peer_id(&modality).unwrap(), peer_id);
    }

    #[test]
    fn other_cids_and_garbage_are_refused() {
        let peer_id = parse_peer_id(BASE58).unwrap();
        let mut dag_pb = vec![CID_V1, 0x70];
        dag_pb.extend_from_slice(&peer_id.to_bytes());
        let dag_pb = multibase::encode(Base::Base32Lower, dag_pb);
        assert!(parse_peer_id(&dag_pb).is_err(), "codec other than libp2p-key");

        for text in ["", "bafz", "hello", "12D3KooW", &BASE58[..40], &BASE32[..40]] {
            assert!(parse_peer_id(text).is_err(), "{text:?}");
        }
    }
}

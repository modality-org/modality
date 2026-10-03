//! The two text forms of a peer ID (a Modality ID is one).
//!
//! libp2p writes a peer ID either as its bare multihash in base58
//! (`12D3KooW…`, the legacy form every implementation prints) or as a CIDv1
//! with the `libp2p-key` codec in a multibase (`bafz…` in base32, `k51…` in
//! base36). Both name the same bytes.
//!
//! Base58 is the only form Modality stores, signs, or compares: rules match
//! signers by string, so a second spelling of one key would be a second
//! signer. Parse what people type with [`parse_peer_id`] and keep
//! [`canonical_peer_id`]'s output. Consensus data (commit signatures, blocks,
//! acks, certificates) accepts base58 only.

use anyhow::{anyhow, bail, Result};
use libp2p_identity::PeerId;
use multibase::Base;

/// CID version 1, as its one-byte varint.
const CID_V1: u8 = 0x01;
/// The `libp2p-key` multicodec, as its one-byte varint.
const LIBP2P_KEY: u8 = 0x72;

/// A peer ID in either text form. Per the libp2p spec, text starting with
/// `1` or `Qm` is a base58 multihash; anything else must be a CIDv1 whose
/// codec is `libp2p-key`.
pub fn parse_peer_id(text: &str) -> Result<PeerId> {
    if text.starts_with('1') || text.starts_with("Qm") {
        return text
            .parse::<PeerId>()
            .map_err(|e| anyhow!("{text} is not a peer ID: {e}"));
    }
    let (_, bytes) =
        multibase::decode(text).map_err(|e| anyhow!("{text} is not a peer ID: {e}"))?;
    match bytes.as_slice() {
        [CID_V1, LIBP2P_KEY, multihash @ ..] => PeerId::from_bytes(multihash)
            .map_err(|e| anyhow!("{text} is not a peer ID: {e}")),
        [CID_V1, ..] => bail!("{text} is a CID, but not of a libp2p key"),
        _ => bail!("{text} is not a peer ID"),
    }
}

/// The base58 form of a peer ID given in either text form.
pub fn canonical_peer_id(text: &str) -> Result<String> {
    parse_peer_id(text).map(|peer_id| peer_id.to_base58())
}

/// `text` in base58 if it is a peer ID in CID form; otherwise `text` as
/// given, for IDs that need not be peer IDs (hub contracts are `c_…`) and
/// for the caller to refuse what is neither.
pub fn normalize_peer_id(text: &str) -> String {
    canonical_peer_id(text).unwrap_or_else(|_| text.to_string())
}

/// The CIDv1 form of `peer_id`, in base32 as the libp2p spec recommends.
pub fn peer_id_to_cid(peer_id: &PeerId) -> String {
    let mut bytes = vec![CID_V1, LIBP2P_KEY];
    bytes.extend_from_slice(&peer_id.to_bytes());
    multibase::encode(Base::Base32Lower, bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    // An ed25519 Modality ID and its CIDv1 forms, from the libp2p spec's
    // encoding (computed independently of this module).
    const BASE58: &str = "12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd";
    const BASE32: &str = "bafzaajaiaejcaabcornvmd5g5bolzmcr6smsje7owdd3xk7lojqydmthdt47iqmi";

    #[test]
    fn both_forms_name_the_same_peer() {
        assert_eq!(canonical_peer_id(BASE58).unwrap(), BASE58);
        assert_eq!(canonical_peer_id(BASE32).unwrap(), BASE58);
        assert_eq!(peer_id_to_cid(&parse_peer_id(BASE58).unwrap()), BASE32);
    }

    #[test]
    fn normalizing_rewrites_only_cids() {
        assert_eq!(normalize_peer_id(BASE32), BASE58);
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

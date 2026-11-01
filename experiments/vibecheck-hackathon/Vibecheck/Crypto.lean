import Vibecheck.Crypto.Ed25519
import Std.Data.HashMap

/-!
Keys and signatures, as a contract sees them: a public key and a signature,
each as hex.

Two schemes:

- `ed25519`, the scheme Modality IDs use: anyone can verify a signature
  with the public key alone.
- `oracle`, ideal signatures for experiments. A signature is a tag, the hash
  of the signer's secret seed and the message, and a contract verifies it by
  asking an oracle that holds the secret of every key it was given. Nobody
  without the seed can produce a tag that verifies, so forgeries and replays
  are refused just as with ed25519; what is lost is public verifiability,
  since verifying means trusting the oracle. Each signature costs one hash.
-/

namespace Vibecheck

structure PublicKey where
  hex : String
  deriving DecidableEq, Hashable, Repr, Inhabited

instance : ToString PublicKey := ⟨fun k => k.hex.take 8⟩

inductive SchemeKind where
  | ed25519
  | oracle
  deriving DecidableEq, Repr, Inhabited

def SchemeKind.name : SchemeKind → String
  | .ed25519 => "ed25519"
  | .oracle => "oracle"

def SchemeKind.ofName? : String → Option SchemeKind
  | "ed25519" => some .ed25519
  | "oracle" => some .oracle
  | _ => none

/-- An oracle tag: the hash of the seed and the message. -/
def oracleTag (seed : ByteArray) (msg : String) : String :=
  Crypto.toHex ((Crypto.sha512 (seed ++ msg.toUTF8)).extract 0 32)

structure Keypair where
  seed : ByteArray
  publicKey : PublicKey
  kind : SchemeKind

namespace Keypair

def ofSeed (seed : ByteArray) (kind : SchemeKind := .ed25519) : Keypair :=
  let pk := match kind with
    | .ed25519 => Crypto.toHex (Crypto.Ed25519.publicKey seed)
    | .oracle => oracleTag seed "oracle public key"
  { seed, publicKey := ⟨pk⟩, kind }

/-- A key derived from a phrase. For demos and tests: anyone who knows the
phrase has the key. -/
def ofPhrase (phrase : String) (kind : SchemeKind := .ed25519) : Keypair :=
  ofSeed ((Crypto.sha512Str phrase).extract 0 32) kind

def sign (k : Keypair) (msg : String) : String :=
  match k.kind with
  | .ed25519 => Crypto.toHex (Crypto.Ed25519.sign k.seed msg.toUTF8)
  | .oracle => oracleTag k.seed msg

end Keypair

/-- How a contract checks a signature: `verify key message signature`. -/
structure Verifier where
  name : String
  verify : PublicKey → String → String → Bool

namespace Verifier

def ed25519 : Verifier where
  name := "ed25519"
  verify k msg sig :=
    match Crypto.ofHex? k.hex, Crypto.ofHex? sig with
    | some pk, some s => Crypto.Ed25519.verify pk msg.toUTF8 s
    | _, _ => false

/-- An oracle that knows the secret behind each of `keys`, and no other. -/
def oracle (keys : List Keypair) : Verifier :=
  let secrets : Std.HashMap PublicKey ByteArray :=
    keys.foldl (fun m k => if k.kind == .oracle then m.insert k.publicKey k.seed else m) {}
  { name := "oracle"
    verify := fun k msg sig =>
      match secrets.get? k with
      | some seed => oracleTag seed msg == sig
      | none => false }

/-- The verifier for keys of `kind`: an oracle needs to be given the keys. -/
def forKind (kind : SchemeKind) (keys : List Keypair) : Verifier :=
  match kind with
  | .ed25519 => ed25519
  | .oracle => oracle keys

end Verifier

end Vibecheck

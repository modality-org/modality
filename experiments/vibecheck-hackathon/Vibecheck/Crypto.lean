import Vibecheck.Crypto.Ed25519

/-!
Keys and signatures, as a contract sees them: an ed25519 public key as hex
(one of the forms a Modality ID takes), and a signature as hex.
-/

namespace Vibecheck

structure PublicKey where
  hex : String
  deriving DecidableEq, Hashable, Repr, Inhabited

instance : ToString PublicKey := ⟨fun k => k.hex.take 8⟩

structure Keypair where
  seed : ByteArray
  publicKey : PublicKey

namespace Keypair

def ofSeed (seed : ByteArray) : Keypair :=
  { seed, publicKey := ⟨Crypto.toHex (Crypto.Ed25519.publicKey seed)⟩ }

/-- A key derived from a phrase. For demos and tests: anyone who knows the
phrase has the key. -/
def ofPhrase (phrase : String) : Keypair := ofSeed ((Crypto.sha512Str phrase).extract 0 32)

def sign (k : Keypair) (msg : String) : String :=
  Crypto.toHex (Crypto.Ed25519.sign k.seed msg.toUTF8)

end Keypair

def PublicKey.verifies (k : PublicKey) (msg : String) (sig : String) : Bool :=
  match Crypto.ofHex? k.hex, Crypto.ofHex? sig with
  | some pk, some s => Crypto.Ed25519.verify pk msg.toUTF8 s
  | _, _ => false

end Vibecheck

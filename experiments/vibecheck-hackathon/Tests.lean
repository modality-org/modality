import Vibecheck
import Vibecheck.Chess

/-!
The checker against brute force. Random small models and formulas; for each,
every lasso trace up to a bound, with every atom value its labels allow, is
evaluated exactly. A model the checker passes must have no violating lasso
(soundness), and a counterexample it reports must replay as a violation.
A violation the checker misses is a bug; a model it refuses whose violation
needs a longer lasso than the bound is only noted.

Run: `lake exe tests`
-/

open Vibecheck

namespace Tests

abbrev P := Pred Unit Unit

def atomNames : Array String := #["a", "b"]
def atomPred (n : String) : P := ⟨n, fun _ => .unknown⟩

/-- A small linear congruential generator. -/
structure Gen where
  s : UInt64

def Gen.next (g : Gen) (n : Nat) : Nat × Gen :=
  let s := g.s * 6364136223846793005 + 1442695040888963407
  ((s >>> 33).toNat % n, ⟨s⟩)

def randFormula (g : Gen) : Nat → LTL String × Gen
  | 0 =>
    let (k, g) := g.next 4
    (match k with | 0 => .atom "a" | 1 => .atom "b" | 2 => .tt | _ => .not (.atom "a"), g)
  | d + 1 =>
    let (k, g) := g.next 8
    match k with
    | 0 => let (φ, g) := randFormula g d; (.not φ, g)
    | 1 => let (φ, g) := randFormula g d; let (ψ, g) := randFormula g d; (.and φ ψ, g)
    | 2 => let (φ, g) := randFormula g d; let (ψ, g) := randFormula g d; (.or φ ψ, g)
    | 3 => let (φ, g) := randFormula g d; (.next φ, g)
    | 4 => let (φ, g) := randFormula g d; let (ψ, g) := randFormula g d; (.U φ ψ, g)
    | 5 => let (φ, g) := randFormula g d; (LTL.always φ, g)
    | 6 => let (φ, g) := randFormula g d; (LTL.eventually φ, g)
    | _ => randFormula g 0

def randModel (g : Gen) : Model Unit Unit × Gen := Id.run do
  let (nq, g) := g.next 3
  let nq := nq + 1
  let (ne, g) := g.next 4
  let mut g := g
  let mut edges := []
  for _ in [0:ne + 1] do
    let (s, g1) := g.next nq
    let (d, g2) := g1.next nq
    let (la, g3) := g2.next 3
    let (lb, g4) := g3.next 3
    g := g4
    let lit (n : String) (k : Nat) : List (Lit Unit Unit) :=
      match k with | 0 => [] | 1 => [⟨atomPred n, true⟩] | _ => [⟨atomPred n, false⟩]
    edges := edges ++ [⟨s, d, lit "a" la ++ lit "b" lb⟩]
  return ({ edges }, g)

/-- Exact value of `φ` at each position of `stem ++ cycle^ω`; `succ` loops the
last position back to `loopAt`. -/
def lassoEval (vals : Array (String → Bool)) (loopAt : Nat) : LTL String → Array Bool
  | .tt => vals.map fun _ => true
  | .ff => vals.map fun _ => false
  | .atom a => vals.map (· a)
  | .not φ => (lassoEval vals loopAt φ).map (!·)
  | .and φ ψ => (lassoEval vals loopAt φ).zipWith (lassoEval vals loopAt ψ) (· && ·)
  | .or φ ψ => (lassoEval vals loopAt φ).zipWith (lassoEval vals loopAt ψ) (· || ·)
  | .next φ =>
    let v := lassoEval vals loopAt φ
    (Array.range vals.size).map fun i => v[if i + 1 < vals.size then i + 1 else loopAt]!
  | .U φ ψ => Id.run do
    let f := lassoEval vals loopAt φ
    let g := lassoEval vals loopAt ψ
    let n := vals.size
    let mut v := Array.mkArray n false
    for _ in [0:n + 1] do
      for i' in [0:n] do
        let i := n - 1 - i'
        let s := if i + 1 < n then i + 1 else loopAt
        v := v.set! i (g[i]! || (f[i]! && v[s]!))
    return v

/-- Every lasso from `q0` with at most `bound` steps: a path and a loop
point, with atom values the labels allow. -/
def lassos (m : Model Unit Unit) (bound : Nat) : List (Array (String → Bool) × Nat) := Id.run do
  let valuations : List (String → Bool) :=
    [fun _ => false, fun n => n == "a", fun n => n == "b", fun _ => true]
  let allows (e : Edge Unit Unit) (v : String → Bool) := e.label.all fun l => v l.pred.name == l.value
  let mut out := []
  -- Paths as (edges, values).
  let mut paths : List (List (Edge Unit Unit) × List (String → Bool)) := [([], [])]
  for _ in [0:bound] do
    let mut next := []
    for (es, vs) in paths do
      let q := match es.getLast? with | some e => e.dst | none => m.initial
      for e in m.edges do
        if e.src == q then
          for v in valuations do
            if allows e v then next := (es ++ [e], vs ++ [v]) :: next
    paths := next
    for (es, vs) in paths do
      let last := es.getLast!.dst
      for j in [0:es.length] do
        if es[j]!.src == last then out := (vs.toArray, j) :: out
  return out

def run : IO UInt32 := do
  let mut g : Gen := ⟨2026⟩
  let mut cases := 0
  let mut holds := 0
  let mut unsound := 0
  let mut unconfirmed := 0
  for _ in [0:3000] do
    let (m, g1) := randModel g
    let (d, g2) := g1.next 4
    let (φ, g3) := randFormula g2 d
    g := g3
    let v := Check.check ([] : Theory Unit Unit) m [m.initial] (φ.map atomPred)
    cases := cases + 1
    let bad := (lassos m 4).filter fun (vals, j) => !(lassoEval vals j φ)[0]!
    match v with
    | .holds =>
      holds := holds + 1
      unless bad.isEmpty do
        unsound := unsound + 1
        IO.println s!"UNSOUND: {φ} on\n{m}"
    | .violated _ =>
      if bad.isEmpty then unconfirmed := unconfirmed + 1
    | .blocks _ => pure ()
    | .tooLarge why => IO.println s!"too large: {why}"
  IO.println s!"{cases} cases: {holds} hold, {unsound} unsound, {unconfirmed} refusals with no violating lasso within 4 steps"
  -- Move generation against known perft totals.
  let start := Chess.perft Chess.Position.start 4
  let kiwi := (Chess.Position.ofFen? Chess.kiwipeteFen).map (Chess.perft · 3)
  let perftOk := start == 197281 && kiwi == some 97862
  IO.println s!"perft: start depth 4 = {start}, kiwipete depth 3 = {kiwi.getD 0}: {if perftOk then "ok" else "WRONG"}"
  -- SHA-512 and RFC 8032 vectors, and tampering.
  let hex (b : ByteArray) := Crypto.toHex b
  let un (h : String) := (Crypto.ofHex? h).getD ByteArray.empty
  let flip (b : ByteArray) (i : Nat) := b.set! i (b.get! i ^^^ 1)
  let seed1 := un "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"
  let seed2 := un "4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb"
  let pk2 := Crypto.Ed25519.publicKey seed2
  let sig2 := Crypto.Ed25519.sign seed2 (un "72")
  let s2 := Crypto.leNat (sig2.extract 32 64)
  let malleable := sig2.extract 0 32 ++ Crypto.natLe (s2 + Crypto.Ed25519.L) 32
  let oa := Keypair.ofPhrase "a" .oracle
  let ob := Keypair.ofPhrase "b" .oracle
  let stranger := Keypair.ofPhrase "c" .oracle
  let oracle := Verifier.oracle [oa, ob]
  let crypto := [
    ("sha512 abc", hex (Crypto.sha512Str "abc") ==
      "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"),
    ("rfc8032 1 key", hex (Crypto.Ed25519.publicKey seed1) ==
      "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a"),
    ("rfc8032 1 sig", hex (Crypto.Ed25519.sign seed1 ByteArray.empty) ==
      "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b"),
    ("rfc8032 2 sig", hex sig2 ==
      "92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00"),
    ("verifies", Crypto.Ed25519.verify pk2 (un "72") sig2),
    ("other message refused", !Crypto.Ed25519.verify pk2 (un "73") sig2),
    ("flipped R refused", !Crypto.Ed25519.verify pk2 (un "72") (flip sig2 3)),
    ("flipped S refused", !Crypto.Ed25519.verify pk2 (un "72") (flip sig2 40)),
    ("other key refused", !Crypto.Ed25519.verify (Crypto.Ed25519.publicKey seed1) (un "72") sig2),
    ("S + L refused", !Crypto.Ed25519.verify pk2 (un "72") malleable),
    ("oracle verifies", oracle.verify oa.publicKey "m" (oa.sign "m")),
    ("oracle: other message refused", !oracle.verify oa.publicKey "n" (oa.sign "m")),
    ("oracle: other key's tag refused", !oracle.verify oa.publicKey "m" (ob.sign "m")),
    ("oracle: unknown key refused", !oracle.verify stranger.publicKey "m" (stranger.sign "m")),
    ("ed25519 verifier: oracle tag refused", !Verifier.ed25519.verify oa.publicKey "m" (oa.sign "m"))]
  for (name, ok) in crypto do
    unless ok do IO.println s!"crypto: {name} FAILED"
  let cryptoOk := crypto.all (·.2)
  IO.println s!"crypto: {(crypto.filter (·.2)).length}/{crypto.length} vectors and tamper checks pass"
  return if unsound == 0 && perftOk && cryptoOk then 0 else 1

end Tests

def main : IO UInt32 := Tests.run

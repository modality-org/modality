import Vibecheck.Crypto.Sha512

/-!
Ed25519 (RFC 8032), the signature scheme Modality IDs use.

Points are in extended twisted Edwards coordinates over GF(2^255 - 19), as
Lean `Nat`s. Fixed-base multiplication uses a table of the base point's
doublings. Verification is the cofactorless check `[S]B = R + [k]A`, which
RFC 8032 allows.
-/

namespace Vibecheck.Crypto.Ed25519

def p : Nat := 2 ^ 255 - 19
/-- The order of the base point. -/
def L : Nat := 2 ^ 252 + 27742317777372353535851937790883648493

def powMod (b e m : Nat) : Nat := Id.run do
  let mut r := 1
  let mut b := b % m
  let mut e := e
  while e > 0 do
    if e % 2 == 1 then r := r * b % m
    b := b * b % m
    e := e / 2
  return r

def inv (x : Nat) : Nat := powMod x (p - 2) p

def d : Nat := (p - 121665) * inv 121666 % p
def d2 : Nat := 2 * d % p
/-- A square root of -1. -/
def sqrtM1 : Nat := powMod 2 ((p - 1) / 4) p

/-- (X : Y : Z : T) with x = X/Z, y = Y/Z, xy = T/Z. -/
structure Point where
  X : Nat
  Y : Nat
  Z : Nat
  T : Nat
  deriving Inhabited

def identity : Point := ⟨0, 1, 1, 0⟩

/-- The unified addition law: also doubles. -/
def add (P Q : Point) : Point :=
  let a := (P.Y + p - P.X) * (Q.Y + p - Q.X) % p
  let b := (P.Y + P.X) * (Q.Y + Q.X) % p
  let c := P.T * d2 % p * Q.T % p
  let dd := 2 * P.Z * Q.Z % p
  let e := b + p - a
  let f := dd + p - c
  let g := dd + c
  let h := b + a
  ⟨e * f % p, g * h % p, f * g % p, e * h % p⟩

def mul (s : Nat) (P : Point) : Point := Id.run do
  let mut q := identity
  let mut a := P
  let mut s := s
  while s > 0 do
    if s % 2 == 1 then q := add q a
    a := add a a
    s := s / 2
  return q

def equal (P Q : Point) : Bool :=
  (P.X * Q.Z + p * p - Q.X * P.Z) % p == 0 && (P.Y * Q.Z + p * p - Q.Y * P.Z) % p == 0

/-- `x` from `y` and the sign bit, when the point exists. -/
def recoverX (y : Nat) (sign : Bool) : Option Nat := do
  if y ≥ p then none
  let x2 := (y * y + p - 1) * inv (d * y * y + 1) % p
  if x2 == 0 then
    if sign then none else return 0
  let mut x := powMod x2 ((p + 3) / 8) p
  if (x * x + p - x2) % p != 0 then x := x * sqrtM1 % p
  if (x * x + p - x2) % p != 0 then none
  if (x % 2 == 1) != sign then x := p - x
  return x

def base : Point :=
  let y := 4 * inv 5 % p
  let x := (recoverX y false).getD 0
  ⟨x, y, 1, x * y % p⟩

/-- `2^i B` for i < 256. -/
def baseTable : Array Point := Id.run do
  let mut t := #[base]
  for _ in [1:256] do t := t.push (add t.back! t.back!)
  return t

def mulBase (s : Nat) : Point := Id.run do
  let mut q := identity
  let mut s := s
  let mut i := 0
  while s > 0 do
    if s % 2 == 1 then q := add q baseTable[i]!
    s := s / 2
    i := i + 1
  return q

def encode (P : Point) : ByteArray :=
  let zi := inv P.Z
  let x := P.X * zi % p
  let y := P.Y * zi % p
  natLe (y + (if x % 2 == 1 then 2 ^ 255 else 0)) 32

def decode? (b : ByteArray) : Option Point := do
  if b.size != 32 then none
  let n := leNat b
  let y := n % 2 ^ 255
  let x ← recoverX y (n / 2 ^ 255 == 1)
  return ⟨x, y, 1, x * y % p⟩

def hashInt (b : ByteArray) : Nat := leNat (sha512 b)

/-- The secret scalar and the nonce prefix of a 32-byte seed. -/
def expand (seed : ByteArray) : Nat × ByteArray :=
  let h := sha512 seed
  let a := leNat (h.extract 0 32)
  let a := (a % 2 ^ 254) - (a % 8) + 2 ^ 254
  (a, h.extract 32 64)

def publicKey (seed : ByteArray) : ByteArray := encode (mulBase (expand seed).1)

def sign (seed : ByteArray) (msg : ByteArray) : ByteArray :=
  let (a, pre) := expand seed
  let pk := encode (mulBase a)
  let r := hashInt (pre ++ msg) % L
  let R := encode (mulBase r)
  let k := hashInt (R ++ pk ++ msg) % L
  R ++ natLe ((r + k * a) % L) 32

def verify (pk : ByteArray) (msg : ByteArray) (sig : ByteArray) : Bool := Id.run do
  if sig.size != 64 then return false
  let some A := decode? pk | return false
  let Rb := sig.extract 0 32
  let some R := decode? Rb | return false
  let s := leNat (sig.extract 32 64)
  if s ≥ L then return false
  let k := hashInt (Rb ++ pk ++ msg) % L
  return equal (mulBase s) (add R (mul k A))

end Vibecheck.Crypto.Ed25519

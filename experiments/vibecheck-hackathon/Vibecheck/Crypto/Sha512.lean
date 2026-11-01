/-!
SHA-512 (FIPS 180-4), and the hex and byte helpers the rest of the crypto
uses.

The round constants and initial hash are computed, as the standard defines
them, from the cube and square roots of the first primes, so there is no
table to mistype.
-/

namespace Vibecheck.Crypto

/-- Floor of the `k`-th root of `x`. -/
def iroot (k x : Nat) : Nat := Id.run do
  let mut r := 0
  for i' in [0:x.log2 / k + 2] do
    let i := x.log2 / k + 1 - i'
    if (r + 2 ^ i) ^ k ≤ x then r := r + 2 ^ i
  return r

def firstPrimes (n : Nat) : Array Nat := Id.run do
  let mut ps := #[]
  let mut c := 2
  while ps.size < n do
    if ps.all fun p => c % p != 0 then ps := ps.push c
    c := c + 1
  return ps

/-- The first 64 bits of the fractional parts of the cube roots of the first
80 primes. -/
def roundK : Array UInt64 := (firstPrimes 80).map fun p => (iroot 3 (p * 2 ^ 192) % 2 ^ 64).toUInt64

/-- The same for the square roots of the first 8 primes. -/
def initialH : Array UInt64 := (firstPrimes 8).map fun p => (iroot 2 (p * 2 ^ 128) % 2 ^ 64).toUInt64

@[inline] def rotr (x : UInt64) (n : UInt64) : UInt64 := (x >>> n) ||| (x <<< (64 - n))

def be64 (b : ByteArray) (i : Nat) : UInt64 := Id.run do
  let mut x : UInt64 := 0
  for j in [0:8] do x := (x <<< 8) ||| (b.get! (i + j)).toUInt64
  return x

def compress (h : Array UInt64) (block : ByteArray) (off : Nat) : Array UInt64 := Id.run do
  let mut w : Array UInt64 := Array.mkEmpty 80
  for t in [0:16] do w := w.push (be64 block (off + 8 * t))
  for t in [16:80] do
    let x := w[t - 15]!
    let y := w[t - 2]!
    let s0 := rotr x 1 ^^^ rotr x 8 ^^^ (x >>> 7)
    let s1 := rotr y 19 ^^^ rotr y 61 ^^^ (y >>> 6)
    w := w.push (w[t - 16]! + s0 + w[t - 7]! + s1)
  let mut a := h[0]!
  let mut b := h[1]!
  let mut c := h[2]!
  let mut d := h[3]!
  let mut e := h[4]!
  let mut f := h[5]!
  let mut g := h[6]!
  let mut hh := h[7]!
  for t in [0:80] do
    let t1 := hh + (rotr e 14 ^^^ rotr e 18 ^^^ rotr e 41) + ((e &&& f) ^^^ (~~~e &&& g)) + roundK[t]! + w[t]!
    let t2 := (rotr a 28 ^^^ rotr a 34 ^^^ rotr a 39) + ((a &&& b) ^^^ (a &&& c) ^^^ (b &&& c))
    hh := g
    g := f
    f := e
    e := d + t1
    d := c
    c := b
    b := a
    a := t1 + t2
  return #[h[0]! + a, h[1]! + b, h[2]! + c, h[3]! + d, h[4]! + e, h[5]! + f, h[6]! + g, h[7]! + hh]

def sha512 (msg : ByteArray) : ByteArray := Id.run do
  let len := msg.size
  let mut m := msg.push 0x80
  while m.size % 128 != 112 do m := m.push 0
  let bits := len * 8
  for i in [0:16] do m := m.push ((bits >>> (8 * (15 - i))) % 256).toUInt8
  let mut h := initialH
  for blk in [0:m.size / 128] do h := compress h m (128 * blk)
  let mut out := ByteArray.empty
  for x in h do
    for i in [0:8] do out := out.push (x >>> (8 * (7 - i).toUInt64)).toUInt8
  return out

/-! ## Bytes, numbers and hex -/

def hexDigit (n : Nat) : Char := "0123456789abcdef".get ⟨n⟩

def toHex (b : ByteArray) : String :=
  b.foldl (fun s x => (s.push (hexDigit (x.toNat / 16))).push (hexDigit (x.toNat % 16))) ""

def ofHex? (s : String) : Option ByteArray := do
  let digit (c : Char) : Option Nat :=
    if '0' ≤ c && c ≤ '9' then some (c.toNat - '0'.toNat)
    else if 'a' ≤ c && c ≤ 'f' then some (c.toNat - 'a'.toNat + 10)
    else if 'A' ≤ c && c ≤ 'F' then some (c.toNat - 'A'.toNat + 10)
    else none
  let cs := s.toList.toArray
  if cs.size % 2 != 0 then none
  let mut out := ByteArray.empty
  for i in [0:cs.size / 2] do
    out := out.push (16 * (← digit cs[2 * i]!) + (← digit cs[2 * i + 1]!)).toUInt8
  return out

/-- Little-endian bytes as a number. -/
def leNat (b : ByteArray) : Nat := b.data.foldr (fun x n => n * 256 + x.toNat) 0

/-- A number as `n` little-endian bytes. -/
def natLe (x n : Nat) : ByteArray := Id.run do
  let mut out := ByteArray.empty
  let mut x := x
  for _ in [0:n] do
    out := out.push (x % 256).toUInt8
    x := x / 256
  return out

def sha512Str (s : String) : ByteArray := sha512 s.toUTF8

end Vibecheck.Crypto

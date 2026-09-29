import PredicateTheory.Fragment

/-!
Declarations: how a predicate on a label becomes literals.

A predicate reaches the theory only through a declaration: an
s-expression template (`(> $1 $2)`) for what it needs (`necessary`) and
what guarantees it (`sufficient`), and a signature saying which argument
kinds the template describes. `expand` turns a label into literals:

- `+P`: the `necessary` atoms, positive.
- `-P`: the single `sufficient` atom, negated (a multi-atom sufficient
  negates to a disjunction, so `-P` stays opaque).
- no usable declaration, or arguments outside the signature: one opaque
  literal carrying the name and arguments.
- a static label (`+POST`): a `label` literal.

`exact` says the literals mean the predicate, in both directions: then
a witness for the literals is a witness for the labels.

Everything here is structural on character lists, so `decide` evaluates
it in the kernel. The Rust twins are `fragment.rs`, `decl.rs`, and
`standard.rs`; change this file first.
-/

namespace PredicateTheory

/-! ## Strings, paths, numbers -/

def splitOnChar (c : Char) : List Char → List (List Char)
  | [] => [[]]
  | x :: xs =>
    match splitOnChar c xs with
    | [] => [[]]
    | seg :: rest => if x == c then [] :: seg :: rest else (x :: seg) :: rest

/-- `norm_path` then split: leading slashes stripped, nothing else. -/
def normPath (s : String) : Path :=
  (splitOnChar '/' (s.toList.dropWhile (· == '/'))).map String.mk

def startsWithSlash (s : String) : Bool := s.toList.head? == some '/'

/-- Extension of the last segment: after its last `.`, if non-empty. -/
def ext (p : Path) : Option String :=
  match p.getLast? with
  | none => none
  | some seg =>
    match (splitOnChar '.' seg.toList).reverse with
    | e :: _ :: _ => if e.isEmpty then none else some (String.mk e)
    | _ => none

def digitVal (c : Char) : Option Nat :=
  if '0' ≤ c && c ≤ '9' then some (c.toNat - '0'.toNat) else none

def digits : List Char → Option Nat
  | cs => cs.foldlM (fun acc c => (digitVal c).map (acc * 10 + ·)) 0

/-- `str::parse::<u32>` / `::<usize>`: optional `+`, at least one digit. -/
def parseNatBounded (bound : Nat) (s : String) : Option Nat :=
  let cs := match s.toList with
    | '+' :: rest => rest
    | cs => cs
  if cs.isEmpty then none
  else match digits cs with
    | some n => if n < bound then some n else none
    | none => none

def parseU32 : String → Option Nat := parseNatBounded (2 ^ 32)
def parseUsize : String → Option Nat := parseNatBounded (2 ^ 64)

/-- `i128::MAX + 1`. -/
def i128Bound : Nat := 2 ^ 127

/-- `Rational::parse`: optional sign, digits, optional fraction; at most
15 significant digits; numerator and denominator fit `i128`. -/
def parseDecimal (s : String) : Option Q :=
  let (neg, body) := match s.toList with
    | '-' :: rest => (true, rest)
    | '+' :: rest => (false, rest)
    | cs => (false, cs)
  if body.isEmpty then none else
  let (ip, fp) := match splitOnChar '.' body with
    | [i] => (i, [])
    | [i, f] => (i, f)
    | _ => ([], ['x'])
  if ip.isEmpty && fp.isEmpty then none else
  let all := ip ++ fp
  let significant := ((all.dropWhile (· == '0')).reverse.dropWhile (· == '0'))
  if significant.length > 15 then none else
  match digits ip, digits fp with
  | some _, some _ =>
    match digits all with
    | some n =>
      let den := 10 ^ fp.length
      if n < i128Bound && den < i128Bound then
        some ⟨if neg then -(n : Int) else n, den - 1⟩
      else none
    | none => none
  | _, _ => none

/-! ## S-expressions -/

inductive SExpr
  | atom (s : String)
  | str (s : String)
  | list (xs : List SExpr)
  deriving Repr

mutual
def SExpr.beq : SExpr → SExpr → Bool
  | .atom a, .atom b => a == b
  | .str a, .str b => a == b
  | .list xs, .list ys => SExpr.beqList xs ys
  | _, _ => false
def SExpr.beqList : List SExpr → List SExpr → Bool
  | [], [] => true
  | x :: xs, y :: ys => SExpr.beq x y && SExpr.beqList xs ys
  | _, _ => false
end

instance : BEq SExpr := ⟨SExpr.beq⟩

inductive Tok
  | lp
  | rp
  | str (s : String)
  | atom (s : String)
  deriving DecidableEq, Repr

def isDelim (c : Char) : Bool := c.isWhitespace || c == '(' || c == ')' || c == '"'

/-- Inside an atom, inside a string, or between tokens. Chars reversed. -/
inductive Mode
  | normal
  | atom (acc : List Char)
  | str (acc : List Char)

/-- One character between tokens. -/
def onNormal (c : Char) (acc : List Tok) : Mode × List Tok :=
  if c.isWhitespace then (.normal, acc)
  else if c == '(' then (.normal, .lp :: acc)
  else if c == ')' then (.normal, .rp :: acc)
  else if c == '"' then (.str [], acc)
  else (.atom [c], acc)

/-- Characters to tokens (reversed accumulator). `none` on an unterminated
string. Strings have no escapes. -/
def tokAux : List Char → Mode → List Tok → Option (List Tok)
  | [], .normal, acc => some acc.reverse
  | [], .atom s, acc => some (Tok.atom (String.mk s.reverse) :: acc).reverse
  | [], .str _, _ => none
  | c :: cs, .normal, acc =>
    let (m, acc) := onNormal c acc
    tokAux cs m acc
  | c :: cs, .str s, acc =>
    if c == '"' then tokAux cs .normal (Tok.str (String.mk s.reverse) :: acc)
    else tokAux cs (.str (c :: s)) acc
  | c :: cs, .atom s, acc =>
    if isDelim c then
      let (m, acc) := onNormal c (Tok.atom (String.mk s.reverse) :: acc)
      tokAux cs m acc
    else tokAux cs (.atom (c :: s)) acc

/-- Tokens to top-level expressions, with an explicit stack of open lists
(each reversed), as `tokenize` in `fragment.rs` does. -/
def parseToks : List Tok → List (List SExpr) → Option (List SExpr)
  | [], [items] => some items.reverse
  | [], _ => none
  | .lp :: ts, st => parseToks ts ([] :: st)
  | .rp :: ts, top :: parent :: rest => parseToks ts ((SExpr.list top.reverse :: parent) :: rest)
  | .rp :: _, _ => none
  | .str s :: ts, top :: rest => parseToks ts ((SExpr.str s :: top) :: rest)
  | .atom s :: ts, top :: rest => parseToks ts ((SExpr.atom s :: top) :: rest)
  | _ :: _, [] => none

def parseSExprs (src : String) : Option (List SExpr) :=
  (tokAux src.toList .normal []).bind (parseToks · [[]])

/-! ## Templates -/

def SExpr.isTerm : SExpr → Bool
  | .atom _ | .str _ => true
  | .list _ => false

/-- Heads and arities only; typing happens at instantiation. -/
def inGrammar : SExpr → Bool
  | .atom _ => true
  | .str _ => false
  | .list (.atom head :: rest) =>
    match head, rest with
    | "<", [a, b] | "<=", [a, b] | ">", [a, b] | "=", [a, b] => a.isTerm && b.isTerm
    | ">=", [.list [.atom "card", q], n] => q.isTerm && n.isTerm
    | ">=", [a, b] => a.isTerm && b.isTerm
    | "contains", [a, b] | "starts-with", [a, b] | "ends-with", [a, b] => a.isTerm && b.isTerm
    | "not", [a] | "exists", [a] | "signed", [a] | "all-signed", [a] | "writes", [a]
    | "posts", [a] => a.isTerm
    | _, _ => false
  | .list _ => false

/-- A parsed declaration body: its atoms (an `and` is split). -/
abbrev Template := List SExpr

def Template.parse (src : String) : Option Template :=
  match parseSExprs src with
  | some [top] =>
    let atoms := match top with
      | .list (.atom "and" :: rest) => rest
      | other => [other]
    if !atoms.isEmpty && atoms.all inGrammar then some atoms else none
  | _ => none

inductive Arg
  | path (p : Path)
  | lit (s : String)

def resolve (args : List String) : SExpr → Option Arg
  | .str s => some (.lit s)
  | .atom a =>
    match a.toList with
    | '$' :: n =>
      (parseUsize (String.mk n)).bind fun i =>
        if i = 0 then none else
        (args.get? (i - 1)).map fun v => if startsWithSlash v then .path (normPath v) else .lit v
    | _ => some (if startsWithSlash a then .path (normPath a) else .lit a)
  | .list _ => none

def numTerm (args : List String) (e : SExpr) : Option Term :=
  match resolve args e with
  | some (.path p) => if ext p == some "num" then some (.path p) else none
  | some (.lit l) => (parseDecimal l).map .const
  | none => none

def typedPath (args : List String) (allowed : List String) (e : SExpr) : Option Path :=
  match resolve args e with
  | some (.path p) =>
    match ext p with
    | some x => if allowed.contains x then some p else none
    | none => none
  | _ => none

def anyPath (args : List String) (e : SExpr) : Option Path :=
  match resolve args e with
  | some (.path p) => some p
  | _ => none

def litArg (args : List String) (e : SExpr) : Option String :=
  match resolve args e with
  | some (.lit l) => some l
  | _ => none

def textOp? : String → Option TextOp
  | "contains" => some .contains
  | "starts-with" => some .startsWith
  | "ends-with" => some .endsWith
  | _ => none

/-- One atom of a template, instantiated and typed by extension. -/
def elabAtom (args : List String) : SExpr → Option Atom
  | e@(.atom _) => (typedPath args ["bool"] e).map (.isBool · true)
  | .str _ => none
  | .list (.atom head :: rest) =>
    match head, rest with
    | "<", [a, b] => do pure (.order (← numTerm args a) .lt (← numTerm args b))
    | "<=", [a, b] => do pure (.order (← numTerm args a) .le (← numTerm args b))
    | ">", [a, b] => do pure (.order (← numTerm args b) .lt (← numTerm args a))
    | ">=", [.list [.atom "card", q], n] => do
      let pre ← anyPath args q
      let k ← (litArg args n).bind parseU32
      pure (.card pre k)
    | ">=", [.list _, _] => none
    | ">=", [a, b] => do pure (.order (← numTerm args b) .le (← numTerm args a))
    | "=", [a, b] =>
      match numTerm args a, numTerm args b with
      | some l, some r => some (.order l .eq r)
      | _, _ => do
        let p ← typedPath args ["text", "id"] a
        match resolve args b with
        | some (.lit l) => some (.textEq p l)
        | some (.path q) =>
          match ext q with
          | some "text" | some "id" => some (.textEq2 p q)
          | _ => none
        | none => none
    | "not", [a] => (typedPath args ["bool"] a).map (.isBool · false)
    | "exists", [a] => (anyPath args a).map .present
    | "signed", [a] => (typedPath args ["id"] a).map .signed
    | "all-signed", [a] => (anyPath args a).map .allSigned
    | "writes", [a] => (anyPath args a).map .writes
    | "posts", [a] => (anyPath args a).map .posts
    | h, [a, b] => do
      let op ← textOp? h
      pure (.text (← typedPath args ["text"] a) op (← litArg args b))
    | _, _ => none
  | .list _ => none

def Template.instantiate (t : Template) (args : List String) : Option (List Atom) :=
  t.mapM (elabAtom args)

/-! ## Declarations and the standard registry -/

/-- What the evaluator reads at one argument position. -/
inductive Param
  | path | numPath | textPath | boolPath | idPath | num | text | needle | nat | any
  deriving DecidableEq, Repr

def Param.parse : String → Option Param
  | "path" => some .path
  | "num-path" => some .numPath
  | "text-path" => some .textPath
  | "bool-path" => some .boolPath
  | "id-path" => some .idPath
  | "num" => some .num
  | "text" => some .text
  | "needle" => some .needle
  | "nat" => some .nat
  | "any" => some .any
  | _ => none

/-- Decimal syntax, `[+-]digits[.digits]`, at any length (Rust:
`is_decimal`). A decimal outside the exact domain fits; `parseDecimal`
then fails and the predicate stays opaque. -/
def isDecimal (s : String) : Bool :=
  let body := match s.toList with
    | '-' :: r => r
    | '+' :: r => r
    | r => r
  let (i, f) := match body.span (· != '.') with
    | (i, _ :: f) => (i, f)
    | (i, []) => (i, [])
  !(i.isEmpty && f.isEmpty) && i.all Char.isDigit && f.all Char.isDigit

def Param.fits (p : Param) (arg : String) : Bool :=
  let pathExt := fun (allowed : List String) =>
    startsWithSlash arg && match ext (normPath arg) with
      | some e => allowed.contains e
      | none => false
  match p with
  | .path => startsWithSlash arg
  | .numPath => pathExt ["num"]
  | .textPath => pathExt ["text", "id"]
  | .boolPath => pathExt ["bool"]
  | .idPath => pathExt ["id"]
  | .num => pathExt ["num"] || (!startsWithSlash arg && isDecimal arg)
  | .text => pathExt ["text", "id"] || !startsWithSlash arg
  | .needle => true
  | .nat => (parseU32 arg).isSome
  | .any => true

structure Declaration where
  necessary : Option Template
  sufficient : Option Template
  params : Option (List Param)

def Declaration.accepts (d : Declaration) (args : List String) : Bool :=
  match d.params with
  | none => true
  | some ps => ps.enum.all fun (i, p) =>
      match args.get? i with
      | some a => p.fits a
      | none => false

/-- Both directions from text; a direction that fails to parse is dropped. -/
def Declaration.ofText (necessary sufficient : Option String) : Declaration :=
  ⟨necessary.bind Template.parse, sufficient.bind Template.parse, none⟩

/-- Neither direction parsed: `modality/declaration-unparsed`. -/
def Declaration.unparsed (d : Declaration) : Bool := d.necessary.isNone && d.sufficient.isNone

/-- Restrict to a signature; an unknown kind drops the whole declaration. -/
def Declaration.withParams (d : Declaration) (sig : String) : Declaration :=
  let words := (splitOnChar ' ' sig.toList).filter (!·.isEmpty) |>.map String.mk
  match words.mapM Param.parse with
  | some ps => { d with params := some ps }
  | none => ⟨none, none, d.params⟩

/-- `(name, signature, necessary, sufficient)`: `none` sufficient is exact
(the same template both ways), `some ""` is necessary-only. The same
table as `standard.rs`. -/
def standardTable : List (String × String × String × Option String) := [
  ("num_gt", "num-path num", "(> $1 $2)", none),
  ("num_gte", "num-path num", "(>= $1 $2)", none),
  ("num_lt", "num-path num", "(< $1 $2)", none),
  ("num_lte", "num-path num", "(<= $1 $2)", none),
  ("num_eq", "num-path num", "(= $1 $2)", none),
  ("amount_in_range", "num-path num num", "(and (<= $2 $1) (<= $1 $3))", none),
  ("signed_by", "id-path", "(signed $1)", none),
  ("any_signed", "path", "(>= (card $1) 1)", none),
  ("all_signed", "path", "(all-signed $1)", none),
  ("threshold", "nat path", "(>= (card $2) $1)", none),
  ("modifies", "path", "(writes $1)", none),
  ("post_to_path", "path", "(posts $1)", none),
  ("post_to", "path", "(posts $1)", some ""),
  ("posts_own_key", "id-path", "(posts $1)", some ""),
  ("text_eq", "text-path text", "(= $1 $2)", none),
  ("bool_true", "bool-path", "$1", none),
  ("bool_false", "bool-path", "(not $1)", none),
  ("state_exists", "path", "(exists $1)", none),
  ("has_property", "path any", "(exists $1)", some ""),
  ("text_contains", "text-path needle", "(contains $1 $2)", none),
  ("text_starts_with", "text-path needle", "(starts-with $1 $2)", none),
  ("text_ends_with", "text-path needle", "(ends-with $1 $2)", none)
]

def standardDecl (name : String) : Option Declaration :=
  (standardTable.find? (·.1 == name)).map fun (_, sig, nec, suf) =>
    let d := match suf with
      | none => Declaration.ofText (some nec) (some nec)
      | some "" => Declaration.ofText (some nec) none
      | some s => Declaration.ofText (some nec) (some s)
    d.withParams sig

/-- A contract's registry: the standard set, then its committed custom
declarations by module path (a custom one never overrides a standard
name; an unparsed one behaves as opaque). -/
def Registry := String → Option Declaration

def registryWith (custom : List (String × Declaration)) : Registry := fun key =>
  match standardDecl key with
  | some d => some d
  | none =>
    (custom.find? (·.1 == key)).bind fun (_, d) =>
      if d.necessary.isNone && d.sufficient.isNone then none else some d

def standard : Registry := registryWith []

/-- The predicates the validator's evaluator reads (Rust:
`EVALUATED_PREDICATES`, next to `predicate_holds`). Every other predicate
(`after`, `before`, `timestamp_valid`, `hash_matches`, `+wasm(...)`, an
unknown name) never holds on the validator. -/
def evaluated : List String := [
  "signed_by", "any_signed", "all_signed", "threshold", "modifies", "post_to_path",
  "post_to", "has_property", "state_exists", "text_eq", "text_contains", "text_starts_with",
  "text_ends_with", "amount_in_range", "num_eq", "num_gt", "num_gte", "num_lt", "num_lte",
  "bool_true", "bool_false", "oracle_attests", "sent_eq", "sent_lte", "sent_to",
  "posts_own_key", "emitted_by", "keeps_product",
  "keeps_product_per_share", "tracks", "pays_senders"
]

/-- False in every world, both ways. -/
def neverDecl : Declaration := Declaration.ofText (some "(< 0 0)") (some "(< 0 0)")

/-- The validator's registry (Rust: `ValidatorRegistry`): the standard
declaration of each evaluated predicate (none for `oracle_attests`,
`sent_eq`, `sent_lte`, `sent_to` and `emitted_by`, which stay opaque), and
`neverDecl` for every other. A committed `+wasm(...)` declaration binds
nothing while the validator does not evaluate `wasm`. -/
def validator : Registry := fun key =>
  if evaluated.contains key then standardDecl key else some neverDecl

/-! ## Labels -/

/-- A property on a transition label, as written: `+num_gt(/x.num, "5")`,
`-wasm(/m.wasm, /p.num)`, `+POST` (static). -/
structure Label where
  pos : Bool
  name : String
  args : List String
  static : Bool := false
  deriving DecidableEq, Repr

structure Expansion where
  lits : List Lit
  exact : Bool

def Label.key (l : Label) : String × List String :=
  if l.name == "wasm" then
    match l.args with
    | m :: rest => (m, rest)
    | [] => (l.name, l.args)
  else (l.name, l.args)

/-- `0 < 0`: no world satisfies it. -/
def neverAtom : Atom := .order (.const (Q.ofInt 0)) .lt (.const (Q.ofInt 0))

/-- A declared predicate used with an argument of the wrong kind never
holds: the evaluator reads it as false (Rust: `predicate_holds`). -/
def expand (reg : Registry) (l : Label) : Expansion :=
  let (key, args) := l.key
  let opq : Expansion := ⟨[⟨l.pos, .opaque key args⟩], false⟩
  if l.static then ⟨[⟨l.pos, .label l.name⟩], true⟩ else
  match reg key with
  | none => opq
  | some d =>
    if !d.accepts args then ⟨[⟨l.pos, neverAtom⟩], true⟩ else
    let exact := d.sufficient == d.necessary
    if l.pos then
      match d.necessary.bind (·.instantiate args) with
      | some atoms => ⟨atoms.map pos, exact⟩
      | none => opq
    else
      match d.sufficient with
      | some [t] =>
        match elabAtom args t with
        | some a => ⟨[neg a], exact⟩
        | none => opq
      | _ => opq

def expandAll (reg : Registry) (ls : List Label) : Expansion :=
  ls.foldl (fun acc l => let e := expand reg l; ⟨acc.lits ++ e.lits, acc.exact && e.exact⟩) ⟨[], true⟩

end PredicateTheory

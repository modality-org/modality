import PredicateTheory

/-!
`pt-check`: the proven checker as a program, for the Rust-vs-Lean harness.

Reads label sets from stdin, one per line, literals separated by ` ; `:

```
+num_gt /x.num 5 ; -state_exists /x.num
+signed_by /a.id ; +bool_true /f.bool
```

A second argument that starts with `/` is a path; otherwise an integer.
Prints `dead` or `live` for each line (`error` if it does not parse).
-/

open PredicateTheory

def parseTerm (s : String) : Option (Term Int) :=
  if s.startsWith "/" then some (.path s) else s.toInt?.map .const

def parseAtom (name : String) (args : List String) : Option (Atom Int) :=
  match name, args with
  | "num_gt", [p, t] => (parseTerm t).map (num_gt p)
  | "num_gte", [p, t] => (parseTerm t).map (num_gte p)
  | "num_lt", [p, t] => (parseTerm t).map (num_lt p)
  | "num_lte", [p, t] => (parseTerm t).map (num_lte p)
  | "num_eq", [p, t] => (parseTerm t).map (num_eq p)
  | "bool_true", [p] => some (bool_true p)
  | "bool_false", [p] => some (bool_false p)
  | "state_exists", [p] => some (state_exists p)
  | "signed_by", [p] => some (signed_by p)
  | _, _ => none

def parseLit (s : String) : Option (Lit Int) :=
  match (s.trim.splitOn " ").filter (· ≠ "") with
  | head :: args =>
    let name := head.drop 1
    match head.get 0 with
    | '+' => (parseAtom name args).map pos
    | '-' => (parseAtom name args).map neg
    | _ => none
  | [] => none

def verdict (line : String) : String :=
  match (line.splitOn ";").mapM parseLit with
  | some ls => if dead ls then "dead" else "live"
  | none => "error"

def main : IO Unit := do
  let input ← IO.getStdin
  let output ← IO.getStdout
  repeat
    let line ← input.getLine
    if line.isEmpty then break
    let line := line.trimRight
    if !line.isEmpty then
      output.putStrLn (verdict line)

import Lake
open Lake DSL

package «predicate-theory» where
  leanOptions := #[
    ⟨`autoImplicit, false⟩,
    ⟨`relaxedAutoImplicit, false⟩
  ]

lean_lib PredicateTheory where
  roots := #[`PredicateTheory]

/-- The proven checker as a program, for the Rust-vs-Lean harness. -/
lean_exe «pt-check» where
  root := `Main

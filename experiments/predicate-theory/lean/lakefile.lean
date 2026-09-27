import Lake
open Lake DSL

package «predicate-theory» where
  leanOptions := #[
    ⟨`autoImplicit, false⟩,
    ⟨`relaxedAutoImplicit, false⟩
  ]

lean_lib PredicateTheory where
  roots := #[`PredicateTheory]

import Lake
open Lake DSL

package «acme8555-governance» where
  leanOptions := #[
    ⟨`autoImplicit, false⟩,
    ⟨`relaxedAutoImplicit, false⟩
  ]

lean_lib Acme8555 where
  roots := #[`Acme8555]

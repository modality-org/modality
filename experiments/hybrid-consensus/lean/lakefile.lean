import Lake
open Lake DSL

package «hybrid-consensus» where
  leanOptions := #[
    ⟨`autoImplicit, false⟩,
    ⟨`relaxedAutoImplicit, false⟩
  ]

lean_lib HybridConsensus where
  roots := #[`HybridConsensus]

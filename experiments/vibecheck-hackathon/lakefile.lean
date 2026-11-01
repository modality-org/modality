import Lake
open Lake DSL

package «vibecheck» where
  leanOptions := #[
    ⟨`autoImplicit, false⟩,
    ⟨`relaxedAutoImplicit, false⟩
  ]

@[default_target]
lean_lib Vibecheck where
  roots := #[`Vibecheck]

/-- Alice and Bob agree on rules, then trade witness models. -/
@[default_target]
lean_exe «alice-bob» where
  root := `AliceBob

/-- Thirty-two agents, one board: chat against a modal contract. -/
@[default_target]
lean_exe «chess» where
  root := `ChessMain

/-- The LTL checker against brute-force lasso evaluation. -/
lean_exe tests where
  root := `Tests

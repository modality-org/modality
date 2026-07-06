namespace Acme8555

/-- ACME roles (mirrors Modality `/users/*.id` paths). -/
inductive Party where
  | accountHolder
  | certificateAuthority
  deriving DecidableEq, Repr, Inhabited

/-- RFC 8555 §7.1.6 order status (`/order/status.text`). -/
inductive OrderStatus where
  | pending
  | ready
  | processing
  | valid
  | invalid
  deriving DecidableEq, Repr, Inhabited

/-- RFC 8555 §7.1.6 challenge status (`/challenge/status.text`). -/
inductive ChallengeStatus where
  | pending
  | processing
  | valid
  deriving DecidableEq, Repr, Inhabited

/-- Contract-visible path write (mirrors `+sets(path, value)` on transitions). -/
inductive PathWrite where
  | orderStatus (s : OrderStatus)
  | challengeStatus (s : ChallengeStatus)
  | certInUse
  | certRevoked
  deriving DecidableEq, Repr, Inhabited

/-- Opaque issuance witness nodes q0…q5 (mirrors `part issuance`). -/
inductive IssuanceState where
  | q0 | q1 | q2 | q3 | q4 | q5
  deriving DecidableEq, Repr, Inhabited

structure Event where
  writes : List PathWrite
  actor : Party
  deriving DecidableEq, Repr, Inhabited

def Event.hasWrite (e : Event) (w : PathWrite) : Bool :=
  w ∈ e.writes

def traceGet (trace : List Event) (i : Nat) : Option Event :=
  trace[i]?

end Acme8555

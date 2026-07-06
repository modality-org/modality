namespace Acme8555

/-- ACME roles in the governance slice (mirrors Modality `/users/*.id` paths). -/
inductive Party where
  | accountHolder
  | certificateAuthority
  deriving DecidableEq, Repr, Inhabited

/-- RFC 8555 §7.1.6 order status values (mirrors `/order/status.text`). -/
inductive OrderStatus where
  | pending
  | ready
  | processing
  | valid
  | invalid
  deriving DecidableEq, Repr, Inhabited

/-- RFC 8555 §7.1.6 challenge status values (mirrors `/challenge/status.text`). -/
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
  deriving DecidableEq, Repr, Inhabited

/-- Opaque witness LTS nodes — one per order status (mirrors q0…q5). -/
inductive State where
  | q0 | q1 | q2 | q3 | q4 | q5
  deriving DecidableEq, Repr, Inhabited

structure Event where
  writes : List PathWrite
  actor : Party
  deriving DecidableEq, Repr

def Event.hasWrite (e : Event) (w : PathWrite) : Bool :=
  w ∈ e.writes

def hasWrite (trace : List Event) (w : PathWrite) : Prop :=
  ∃ i, i < trace.length ∧ trace[i]?.any (Event.hasWrite · w) = some true

def beforeWrite (trace : List Event) (earlier later : PathWrite) : Prop :=
  ∃ i j,
    i < j ∧
    j < trace.length ∧
    trace[i]?.any (Event.hasWrite · earlier) = some true ∧
    trace[j]?.any (Event.hasWrite · later) = some true

end Acme8555

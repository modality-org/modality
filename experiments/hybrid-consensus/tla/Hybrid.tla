---- MODULE Hybrid ----
(***************************************************************************)
(* Miner-nominated sequencing: composition only.                           *)
(*                                                                         *)
(* A miner block appends (miner, nominee) to a canonical chain.            *)
(* Epoch of height h is (h-1) \div BlocksPerEpoch.                         *)
(* Committee(e) = set of nominees in epoch e - Lookback (empty if e <      *)
(* Lookback). A sequencer step in the current epoch is allowed only if     *)
(* the actor is in that committee.                                         *)
(*                                                                         *)
(* DAG-BFT internals are omitted. This model asks: can we elect sequencers *)
(* from mining without letting the current epoch rewrite its own committee.*)
(***************************************************************************)

EXTENDS Naturals, Sequences, FiniteSets, TLC

CONSTANTS Miners, BlocksPerEpoch, Lookback, MaxHeight

ASSUME BlocksPerEpoch \in Nat \ {0}
ASSUME Lookback \in Nat \ {0}
ASSUME MaxHeight \in Nat \ {0}

VARIABLES chain
  (* Seq of [miner |-> m, nominee |-> n] with m,n \in Miners *)

vars == <<chain>>

Height == Len(chain)

EpochOf(h) == IF h = 0 THEN 0 ELSE (h - 1) \div BlocksPerEpoch

CurrentEpoch == EpochOf(Height)

IdxInEpoch(e) == { i \in DOMAIN chain : EpochOf(i) = e }

Nominations(e) == { chain[i].nominee : i \in IdxInEpoch(e) }

Committee(e) ==
  IF e < Lookback THEN {} ELSE Nominations(e - Lookback)

TypeOK ==
  chain \in Seq([miner : Miners, nominee : Miners])

\* Committee(e) is exactly the nominee set of the lookback epoch.
LookbackCommittee ==
  \A e \in 0..MaxHeight :
    Committee(e) =
      IF e < Lookback THEN {} ELSE Nominations(e - Lookback)

\* A name is in committee e only if some miner block in epoch e - Lookback
\* nominated it. Current-epoch blocks cannot be the source of committee e.
LookbackStable ==
  \A e \in 0..MaxHeight :
    e >= Lookback =>
      \A n \in Committee(e) :
        \E i \in IdxInEpoch(e - Lookback) : chain[i].nominee = n

Mine(m, n) ==
  /\ Height < MaxHeight
  /\ m \in Miners
  /\ n \in Miners
  /\ chain' = Append(chain, [miner |-> m, nominee |-> n])

\* A sequencer step at the current height's epoch. No BFT payload;
\* eligibility is the composition property.
Sequence(s) ==
  /\ Height > 0
  /\ CurrentEpoch >= Lookback
  /\ s \in Committee(CurrentEpoch)
  /\ UNCHANGED chain

Init == chain = << >>

Next ==
  \/ \E m, n \in Miners : Mine(m, n)
  \/ \E s \in Miners : Sequence(s)

Spec == Init /\ [][Next]_vars

Eligibility ==
  \A s \in Miners :
    [][Sequence(s) => s \in Committee(CurrentEpoch)]_vars

====

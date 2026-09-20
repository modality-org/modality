---- MODULE Hybrid ----
(***************************************************************************)
(* Combined consensus: composition of a PoW miner index with a BFT         *)
(* sequencer log. DAG-BFT internals are a cited black box (a Sequence      *)
(* step is one certified commit).                                          *)
(*                                                                         *)
(* Two ledgers:                                                            *)
(*   chain   — canonical miner index (join / nomination). May reorg an     *)
(*             unstable suffix.                                            *)
(*   seqLog  — finalized prefix of contract commits. Append-only; never    *)
(*             rewritten by mining or reorg.                               *)
(*                                                                         *)
(* Committee(e) = nominees from miner epoch e - Lookback (empty if         *)
(* e < Lookback). Fast-finality Sequence is enabled only under             *)
(* NormalConditions. Stall disables Sequence without touching seqLog.      *)
(* Halt (Byzantine committee) freezes sequencing forever in this model.    *)
(***************************************************************************)

EXTENDS Naturals, Sequences, FiniteSets, TLC

CONSTANTS Honest, Byzantine, BlocksPerEpoch, Lookback, MaxHeight, MaxSeq

ASSUME BlocksPerEpoch \in Nat \ {0}
ASSUME Lookback \in Nat \ {0}
ASSUME MaxHeight \in Nat \ {0}
ASSUME MaxSeq \in Nat \ {0}
ASSUME Honest \cap Byzantine = {}

Miners == Honest \cup Byzantine

VARIABLES
  chain,          \* Seq of [miner |-> m, nominee |-> n]
  seqLog,         \* Seq of [author |-> s]  — finalized prefix
  honestHash,     \* NC: adversarial hash below bound (abstracted)
  minerAgree,     \* NC: honest nodes share the lookback prefix
  committeeLive,  \* NC: honest quorum of Committee(e) is awake
  partialSync,    \* NC: GST reached; committee messages within Delta
  halted          \* conflicting certificates seen; no merge

vars == <<chain, seqLog, honestHash, minerAgree, committeeLive, partialSync, halted>>

\* Environment flags (not a TLA VARIABLE). Grouped for reading only.
\* honestHash, minerAgree, committeeLive, partialSync, halted

Height == Len(chain)

EpochOf(h) == IF h = 0 THEN 0 ELSE (h - 1) \div BlocksPerEpoch

CurrentEpoch == EpochOf(Height)

IdxInEpoch(e) == { i \in DOMAIN chain : EpochOf(i) = e }

Nominations(e) == { chain[i].nominee : i \in IdxInEpoch(e) }

Committee(e) ==
  IF e < Lookback THEN {} ELSE Nominations(e - Lookback)

CommitteeExists ==
  /\ CurrentEpoch >= Lookback
  /\ Committee(CurrentEpoch) # {}

\* Vacuous on empty committee (no sequencers to be Byzantine).
CommitteeHonest ==
  IF Committee(CurrentEpoch) = {}
  THEN TRUE
  ELSE 3 * Cardinality(Committee(CurrentEpoch) \cap Byzantine)
         < Cardinality(Committee(CurrentEpoch))

NC_mine == honestHash /\ minerAgree

NormalConditions ==
  /\ NC_mine
  /\ CommitteeExists
  /\ CommitteeHonest
  /\ committeeLive
  /\ partialSync
  /\ ~halted

TypeOK ==
  /\ chain \in Seq([miner : Miners, nominee : Miners])
  /\ seqLog \in Seq([author : Miners])
  /\ honestHash \in BOOLEAN
  /\ minerAgree \in BOOLEAN
  /\ committeeLive \in BOOLEAN
  /\ partialSync \in BOOLEAN
  /\ halted \in BOOLEAN

\* A committee member for e was nominated in epoch e - Lookback.
LookbackStable ==
  \A e \in 0..MaxHeight :
    e >= Lookback =>
      \A n \in Committee(e) :
        \E i \in IdxInEpoch(e - Lookback) : chain[i].nominee = n

\* Current-epoch blocks are not the source of the current committee.
CurrentEpochDoesNotElect ==
  CurrentEpoch >= Lookback =>
    \A i \in IdxInEpoch(CurrentEpoch) :
      chain[i].nominee \in Committee(CurrentEpoch) =>
        \E j \in IdxInEpoch(CurrentEpoch - Lookback) :
          chain[j].nominee = chain[i].nominee

-----------------------------------------------------------------------------
\* Actions — miner index

Mine(m, n) ==
  /\ Height < MaxHeight
  /\ m \in Miners
  /\ n \in Miners
  /\ chain' = Append(chain, [miner |-> m, nominee |-> n])
  /\ UNCHANGED <<seqLog, honestHash, minerAgree, committeeLive, partialSync, halted>>

\* Reorg the current epoch's suffix only (unstable trailing blocks).
\* Lookback epochs and seqLog are untouched. Allowed while NC_mine
\* (honest fork-choice still agrees on the lookback prefix).
ShallowReorg(m, n, k) ==
  /\ honestHash
  /\ minerAgree
  /\ ~halted
  /\ k \in 1..Height
  /\ \A i \in (Height - k + 1)..Height : EpochOf(i) = CurrentEpoch
  /\ m \in Miners
  /\ n \in Miners
  /\ chain' = Append(
                 IF k = Height THEN << >> ELSE SubSeq(chain, 1, Height - k),
                 [miner |-> m, nominee |-> n])
  /\ UNCHANGED <<seqLog, honestHash, minerAgree, committeeLive, partialSync, halted>>

\* Majority-hash (or worse chain quality): the miner index may lose
\* lookback history. The finalized prefix still does not move.
DeepReorg(k) ==
  /\ ~honestHash
  /\ k \in 1..Height
  /\ chain' = IF k = Height THEN << >> ELSE SubSeq(chain, 1, Height - k)
  /\ UNCHANGED <<seqLog, honestHash, minerAgree, committeeLive, partialSync, halted>>

-----------------------------------------------------------------------------
\* Actions — sequencer log (BFT black box)

Sequence(s) ==
  /\ NormalConditions
  /\ s \in Committee(CurrentEpoch)
  /\ Len(seqLog) < MaxSeq
  /\ seqLog' = Append(seqLog, [author |-> s])
  /\ UNCHANGED <<chain, honestHash, minerAgree, committeeLive, partialSync, halted>>

\* Stall: lose liveness conjuncts. No new finals. Prefix unchanged.
LoseLive ==
  /\ committeeLive
  /\ committeeLive' = FALSE
  /\ UNCHANGED <<chain, seqLog, honestHash, minerAgree, partialSync, halted>>

LoseSync ==
  /\ partialSync
  /\ partialSync' = FALSE
  /\ UNCHANGED <<chain, seqLog, honestHash, minerAgree, committeeLive, halted>>

RestoreLive ==
  /\ ~committeeLive
  /\ ~halted
  /\ committeeLive' = TRUE
  /\ UNCHANGED <<chain, seqLog, honestHash, minerAgree, partialSync, halted>>

RestoreSync ==
  /\ ~partialSync
  /\ ~halted
  /\ partialSync' = TRUE
  /\ UNCHANGED <<chain, seqLog, honestHash, minerAgree, committeeLive, halted>>

LoseHash ==
  /\ honestHash
  /\ honestHash' = FALSE
  /\ UNCHANGED <<chain, seqLog, minerAgree, committeeLive, partialSync, halted>>

RestoreHash ==
  /\ ~honestHash
  /\ ~halted
  /\ honestHash' = TRUE
  /\ UNCHANGED <<chain, seqLog, minerAgree, committeeLive, partialSync, halted>>

LoseAgree ==
  /\ minerAgree
  /\ minerAgree' = FALSE
  /\ UNCHANGED <<chain, seqLog, honestHash, committeeLive, partialSync, halted>>

RestoreAgree ==
  /\ ~minerAgree
  /\ ~halted
  /\ minerAgree' = TRUE
  /\ UNCHANGED <<chain, seqLog, honestHash, committeeLive, partialSync, halted>>

\* Byzantine / adaptive capture: this model does not merge two
\* conflicting certificates. Halt sequencing. Prefix frozen as-is.
Equivocate ==
  /\ ~halted
  /\ CommitteeExists
  /\ ~CommitteeHonest
  /\ halted' = TRUE
  /\ UNCHANGED <<chain, seqLog, honestHash, minerAgree, committeeLive, partialSync>>

-----------------------------------------------------------------------------

Init ==
  /\ chain = << >>
  /\ seqLog = << >>
  /\ honestHash = TRUE
  /\ minerAgree = TRUE
  /\ committeeLive = TRUE
  /\ partialSync = TRUE
  /\ halted = FALSE

Next ==
  \/ \E m, n \in Miners : Mine(m, n)
  \/ \E m, n \in Miners, k \in 1..MaxHeight : ShallowReorg(m, n, k)
  \/ \E k \in 1..MaxHeight : DeepReorg(k)
  \/ \E s \in Miners : Sequence(s)
  \/ LoseLive \/ LoseSync \/ RestoreLive \/ RestoreSync
  \/ LoseHash \/ RestoreHash \/ LoseAgree \/ RestoreAgree
  \/ Equivocate

Spec == Init /\ [][Next]_vars

-----------------------------------------------------------------------------
\* Action properties (TLC PROPERTIES). Unprimed = pre-state.

\* Mining / reorg never rewrites the finalized prefix; sequencing never
\* rewrites the miner index.
Separation ==
  /\ [][(chain' # chain) => seqLog' = seqLog]_vars
  /\ [][(seqLog' # seqLog) => chain' = chain]_vars

SeqLogAppendOnly ==
  [][ seqLog' = seqLog
      \/ \E rec \in [author : Miners] : seqLog' = Append(seqLog, rec) ]_vars

\* New finals only under NormalConditions, by a member of the lookback
\* committee, and never after halt.
Eligibility ==
  [][ seqLog' # seqLog =>
        /\ NormalConditions
        /\ \E s \in Committee(CurrentEpoch) :
             seqLog' = Append(seqLog, [author |-> s]) ]_vars

\* Completed miner epochs are not rewritten by a shallow (current-epoch)
\* reorg or by mining the current epoch.
PastEpochsFrozen ==
  [][ \A e \in 0..MaxHeight :
        (e < CurrentEpoch /\ e < CurrentEpoch') =>
          Nominations(e) = Nominations(e)' ]_vars

\* Stall / lost sync / lost agreement / halt: no new finals.
StallFreezesPrefix ==
  [][ (~committeeLive \/ ~partialSync \/ ~minerAgree \/ halted) =>
        seqLog' = seqLog ]_vars

\* After halt, sequencing stays off.
HaltSticky ==
  [][halted => halted']_vars

====

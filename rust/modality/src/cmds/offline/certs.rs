//! `--emit-certificate DIR`: each proof as JSON, one file per obligation.
//!
//! A certificate lists, per case, the facts it combines and the
//! non-negative multipliers that make their sum a contradiction, or the
//! text lemma it applies. `modal prove` checks every arithmetic step again
//! before writing it; the files are for other checkers.

use super::project::Project;
use anyhow::Result;
use modality_lang::prove::solve::{Close, Proof, Src};
use modality_lang::prove::{Obligation, Prover, Verdict};
use serde_json::{json, Value};

fn src(s: &Src) -> Value {
    match s {
        Src::Fact(n) => json!({ "fact": n }),
        Src::Guard(e) => json!({ "guard": e }),
        Src::Type(v) => json!({ "type": v }),
        Src::Goal => json!("goal"),
        Src::Case(c) => json!({ "case": c }),
        Src::Lemma(l) => json!({ "lemma": l }),
    }
}

pub fn proof_json(p: &Proof) -> Value {
    match p {
        Proof::Split { on, cases } => json!({
            "split": on,
            "cases": cases.iter().map(|(c, sub)| json!({ "case": c, "proof": proof_json(sub) })).collect::<Vec<_>>(),
        }),
        Proof::Closed(Close::Arith { rows, cert }) => json!({
            "arithmetic": cert.multipliers.iter().filter(|(_, m)| *m != modality_lang::theory::Rational::from_int(0)).map(|(i, m)| {
                let (row, s, shown) = &rows[*i];
                json!({
                    "fact": shown,
                    "from": src(s),
                    "multiplier": m.to_string(),
                    "tightened": cert.tightened.contains(i),
                    "row": format!("{} {} 0", row.lin.show(&|v| v.to_string()), if row.rel == modality_lang::prove::lin::Rel::Lt { "<" } else { "≤" }),
                })
            }).collect::<Vec<_>>(),
        }),
        Proof::Closed(Close::Direct { src: s, why }) => json!({
            "direct": why,
            "from": s.iter().map(src).collect::<Vec<_>>(),
        }),
        Proof::Closed(Close::Text { lemma, negated, src: s, side, .. }) => json!({
            "lemma": lemma,
            "gives": negated,
            "from": s.iter().map(src).collect::<Vec<_>>(),
            "side": side.iter().map(proof_json).collect::<Vec<_>>(),
        }),
    }
}

pub fn write(p: &Project, _prover: &Prover, obs: &[Obligation], dir: &str) -> Result<()> {
    std::fs::create_dir_all(dir)?;
    for ob in obs {
        let parts: Vec<Value> = ob
            .parts
            .iter()
            .map(|pt| {
                let edge = pt.edge.map(|e| p.machine.edges[e].name());
                match &pt.verdict {
                    Verdict::Proven { proof: Some(pr), .. } => json!({ "part": pt.what, "edge": edge, "proof": proof_json(pr) }),
                    Verdict::Proven { proof: None, note, .. } => json!({ "part": pt.what, "edge": edge, "by": note }),
                    Verdict::Refuted(_) => json!({ "part": pt.what, "edge": edge, "refuted": true }),
                    Verdict::Unknown(w) => json!({ "part": pt.what, "edge": edge, "unknown": w }),
                }
            })
            .collect();
        let doc = json!({
            "obligation": ob.id(),
            "semantics": super::prove::SEMANTICS,
            "prover": super::prove::PROVER,
            "parts": parts,
        });
        let file = ob.id().replace(|c: char| !c.is_alphanumeric() && c != '.' && c != '_', "_");
        std::fs::write(format!("{dir}/{file}.json"), serde_json::to_string_pretty(&doc)?)?;
    }
    Ok(())
}

use modality_lang::data::invariants;
use modality_lang::data::Machine;
use modality_lang::prove::{Prover, Rule, Verdict};

fn rules(src: &str) -> Vec<Rule> {
    let mut out = Vec::new();
    for b in modality_lang::rule_file::parse_rule_file(src).unwrap() {
        for f in b.formulas {
            let name = format!("{}.{}", b.name.clone().unwrap(), f.name.clone().unwrap());
            let wrapped = format!("formula x {{\n{}\n}}", f.body);
            let expr = modality_lang::parse_all_formulas_content_lalrpop(&wrapped).unwrap().remove(0).expression;
            out.push(Rule { name, expr });
        }
    }
    out
}

pub fn prove(model: &str, inv: &str) -> Vec<(String, String)> {
    let dir = "tests/fixtures/leftpad";
    let m = modality_lang::parse_all_models_content_lalrpop(model).unwrap().remove(0);
    let machine = Machine::new(&m).unwrap();
    let inv = invariants::parse(inv).unwrap();
    let rules = rules(&std::fs::read_to_string(format!("{dir}/leftpad.rules.modality")).unwrap());
    let paths = vec![
        "/in/fill.text", "/in/width.integer", "/in/string.text", "/out/result.text",
        "/out/added.natural", "/out/pad.natural", "/out/done.bool",
    ].into_iter().map(String::from).collect();
    let p = Prover::new(&machine, Some(&inv), rules, paths, vec!["0".into()]).unwrap();
    let mut obs = p.obligations();
    p.cross_check(&mut obs);
    obs.iter().map(|o| {
        let v = match o.verdict() {
            Verdict::Proven { .. } => "proven".to_string(),
            Verdict::Refuted(c) => format!("refuted {:?} -> {:?} broken {:?}", c.before, c.after, c.broken),
            Verdict::Unknown(w) => format!("unknown: {w}"),
        };
        (o.id(), v)
    }).collect()
}

#[test]
fn leftpad_proves() {
    let dir = "tests/fixtures/leftpad";
    let model = std::fs::read_to_string(format!("{dir}/leftpad.modality")).unwrap();
    let inv = std::fs::read_to_string(format!("{dir}/leftpad.invariants.modality")).unwrap();
    let r = prove(&model, &inv);
    for (id, v) in &r {
        println!("{id}: {v}");
    }
    assert!(r.iter().all(|(_, v)| v == "proven"), "{r:#?}");
}

#[test]
fn cutting_one_char_fill_refutes_consec() {
    let dir = "tests/fixtures/leftpad";
    let model = std::fs::read_to_string(format!("{dir}/leftpad.modality")).unwrap();
    let inv = std::fs::read_to_string(format!("{dir}/leftpad.invariants.modality")).unwrap();
    let inv = inv.replace("  formula one_char_fill {\n    always([-num_eq(len(/in/fill.text), 1)] false)\n  }\n", "");
    let r = prove(&model, &inv);
    for (id, v) in &r {
        println!("{id}: {v}");
    }
    assert!(r.iter().any(|(id, v)| id == "consec q2 -> q2" && v.starts_with("refuted")));
}

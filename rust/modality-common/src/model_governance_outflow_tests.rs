//! What a pending commit moves and who wrote it: the `SEND`s (`sent_eq`,
//! `sent_lte`, `sent_to`), a key posted by its holder (`posts_own_key`), and
//! actions a posted program emitted (`emitted_by`), and what the commit does
//! to a product of posted numbers (`keeps_product`). Then the faucet, a
//! program-only treasury and a pool's swap invariant, which need them.

use super::*;
use crate::contract_store::{CommitAction, Emitter};
use crate::independent_replay::{expand_invoke_actions, FrozenInvokeContext, InvokeEngine, ReplayWasm};
use serde_json::json;

const V0: TheoryVersion = TheoryVersion::V0;
const V2: TheoryVersion = TheoryVersion::V2;

fn holds(commit: &CommitFile, state: &[(&str, Value)], name: &str, args: &[&str]) -> bool {
    let state: HashMap<String, Value> = state
        .iter()
        .map(|(k, v)| (k.trim_start_matches('/').to_string(), v.clone()))
        .collect();
    CommitFacts::from_commit(commit, &state)
        .under(V2)
        .predicate_holds(&Property::new_predicate_from_call_args(
            name.to_string(),
            args.iter().map(|a| a.to_string()).collect(),
        ))
}

fn sends(items: &[(&str, &str, Value)]) -> CommitFile {
    let mut c = CommitFile::new();
    for (asset, to, amount) in items {
        c.add_action(
            "send".to_string(),
            None,
            json!({"asset_id": asset, "to_contract": to, "amount": amount}),
        );
    }
    c
}

fn signed(mut c: CommitFile, keys: &[&str]) -> CommitFile {
    let sigs: serde_json::Map<String, Value> =
        keys.iter().map(|k| (k.to_string(), json!("sig"))).collect();
    c.head.signatures = Some(Value::Object(sigs));
    c
}

fn commit(actions: Vec<(&str, &str, Value)>) -> CommitFile {
    let mut c = CommitFile::new();
    for (method, path, value) in actions {
        c.add_action(method.to_string(), Some(path.to_string()), value);
    }
    c
}

fn rule_commit(formula: &str) -> CommitFile {
    let rule = format!("export default rule {{\n  formula {{\n    {formula}\n  }}\n}}\n");
    commit(vec![
        ("rule", "/rules/r.modality", json!(rule)),
        ("post", "/notes/a.text", json!("a")),
    ])
}

fn validate(accepted: &[CommitFile], pending: &CommitFile, theory: TheoryVersion) -> Result<()> {
    validate_pending_commit_with_theory(
        "",
        accepted,
        pending,
        None,
        None,
        None,
        TheoryActivation::always(theory),
    )
}

fn emitted(mut c: CommitFile, program: &str, sha256: &str) -> CommitFile {
    for action in &mut c.body {
        action.emitted_by = Some(Emitter {
            program: program.to_string(),
            sha256: sha256.to_string(),
        });
    }
    c
}

#[test]
fn sent_predicates_read_what_the_pending_sends_move() {
    let state = [
        ("/config/drip.num", json!(10)),
        ("/config/half.num", json!(2.5)),
        ("/config/drip.text", json!("10")),
        ("/claimants/alice/wallet.text", json!("w1")),
        ("/claimants/alice/wallet.num", json!(1)),
    ];
    let c = sends(&[
        ("drops", "w1", json!(4)),
        ("drops", "w1", json!(6)),
        ("gold", "w2", json!(1)),
    ]);
    let h = |name: &str, args: &[&str]| holds(&c, &state, name, args);

    assert!(h("sent_eq", &["drops", "10"]));
    assert!(h("sent_eq", &["drops", "/config/drip.num"]));
    assert!(!h("sent_eq", &["drops", "9"]));
    assert!(h("sent_lte", &["drops", "10"]));
    assert!(!h("sent_lte", &["drops", "9"]));
    assert!(h("sent_eq", &["gold", "1"]));
    assert!(h("sent_eq", &["silver", "0"]), "nothing sent is zero");
    assert!(h("sent_to", &["drops", "w1"]));
    assert!(h("sent_to", &["drops", "/claimants/alice/wallet.text"]));
    assert!(!h("sent_to", &["gold", "w1"]));
    assert!(h("sent_to", &["silver", "anywhere"]), "no SEND of the asset");

    // Amounts are whole numbers; paths are read by type.
    assert!(!h("sent_eq", &["drops", "10.0"]));
    assert!(!h("sent_lte", &["drops", "/config/half.num"]));
    assert!(!h("sent_eq", &["drops", "/config/drip.text"]));
    assert!(!h("sent_to", &["drops", "/claimants/alice/wallet.num"]));
    assert!(!h("sent_to", &["drops", "/claimants/bob/wallet.text"]));

    // A malformed SEND makes every sent_* predicate fail.
    let mut bad = c.clone();
    bad.add_action("send".to_string(), None, json!({"asset_id": "drops", "to_contract": "w1"}));
    for (name, args) in [
        ("sent_eq", ["drops", "10"]),
        ("sent_lte", ["drops", "100"]),
        ("sent_to", ["drops", "w1"]),
    ] {
        assert!(!holds(&bad, &state, name, &args), "{name}");
    }
}

#[test]
fn posts_own_key_needs_the_posted_key_to_sign() {
    let post = |key: Value, signers: &[&str]| {
        signed(commit(vec![("post", "/claimants/carol.id", key)]), signers)
    };
    let h = |c: &CommitFile, path: &str| holds(c, &[], "posts_own_key", &[path]);
    assert!(h(&post(json!("KEY_C"), &["KEY_C"]), "/claimants/carol.id"));
    assert!(h(&post(json!("KEY_C"), &["KEY_C", "KEY_M"]), "/claimants/carol.id"));
    assert!(!h(&post(json!("KEY_C"), &["KEY_M"]), "/claimants/carol.id"));
    assert!(!h(&post(json!(7), &["KEY_C"]), "/claimants/carol.id"));
    assert!(!h(&post(json!("KEY_C"), &["KEY_C"]), "/claimants/dave.id"), "no post there");
    assert!(!h(&post(json!("KEY_C"), &["KEY_C"]), "/claimants"), "an .id path");
    let two = signed(
        commit(vec![
            ("post", "/claimants/carol.id", json!("KEY_C")),
            ("post", "/claimants/carol.id", json!("KEY_M")),
        ]),
        &["KEY_C"],
    );
    assert!(!h(&two, "/claimants/carol.id"), "every key posted there signs");
}

#[test]
fn emitted_by_holds_only_when_the_program_wrote_every_action() {
    let program = "/__programs__/payout.wasm";
    let out = emitted(sends(&[("drops", "w1", json!(5))]), program, "abc");
    let h = |c: &CommitFile, args: &[&str]| holds(c, &[], "emitted_by", args);
    assert!(h(&out, &[program]));
    assert!(h(&out, &["__programs__/payout.wasm"]));
    assert!(h(&out, &[program, "ABC"]));
    assert!(!h(&out, &[program, "abd"]), "other bytes");
    assert!(!h(&out, &["/__programs__/other.wasm"]));
    assert!(!h(&out, &["/__programs__/payout.text"]), "a .wasm path");
    assert!(!h(&CommitFile::new(), &[program]), "an empty commit emitted nothing");

    let mut mixed = out.clone();
    mixed.add_action("send".to_string(), None, json!({"asset_id": "drops", "to_contract": "me", "amount": 5}));
    assert!(!h(&mixed, &[program]), "a hand-written SEND beside the program's");

    // Provenance is never read from a commit: a posted commit cannot claim it.
    let posted = json!({
        "head": {},
        "body": [{
            "method": "send",
            "path": null,
            "value": {"asset_id": "drops", "to_contract": "me", "amount": 5},
            "emitted_by": {"program": program, "sha256": "abc"}
        }]
    });
    let parsed: CommitFile = serde_json::from_value(posted).unwrap();
    assert!(parsed.body[0].emitted_by.is_none());
    assert!(!h(&parsed, &[program]));
    let reread: CommitFile = serde_json::from_str(&serde_json::to_string(&out).unwrap()).unwrap();
    assert!(reread.body[0].emitted_by.is_none());
    assert_eq!(reread.compute_id().unwrap(), sends(&[("drops", "w1", json!(5))]).compute_id().unwrap());
}

struct PaysOut;

impl InvokeEngine for PaysOut {
    fn execute_invoke(
        &mut self,
        _wasm: &ReplayWasm,
        _args: &Value,
        _ctx: &FrozenInvokeContext,
    ) -> Result<Vec<CommitAction>> {
        Ok(sends(&[("drops", "w1", json!(5))]).body)
    }
}

#[test]
fn expansion_marks_what_the_program_emitted() {
    let wasm = ReplayWasm {
        path: "__programs__/payout.wasm".to_string(),
        sha256: "ABC".to_string(),
        gas_limit: 1,
        bytes_b64: "AA==".to_string(),
    };
    let mut pending = commit(vec![("invoke", "/__programs__/payout.wasm", json!({"args": {}}))]);
    pending.add_action("post".to_string(), Some("/notes/n.text".to_string()), json!("n"));
    let ctx = crate::independent_replay::frozen_invoke_context("c", "p", &pending, &[]);
    let (expanded, count) = expand_invoke_actions(&pending, &[wasm], &ctx, &mut PaysOut).unwrap();
    assert_eq!(count, 1);
    assert_eq!(expanded.body[0].method, "send");
    assert_eq!(
        expanded.body[0].emitted_by,
        Some(Emitter {
            program: "/__programs__/payout.wasm".to_string(),
            sha256: "abc".to_string(),
        })
    );
    assert_eq!(expanded.body[1].method, "post");
    assert!(expanded.body[1].emitted_by.is_none(), "written by hand");
}

// ---------------------------------------------------------------------------
// The faucet v1: a fixed drip, one per claimant, keys registered by holders
// ---------------------------------------------------------------------------

/// Anyone registers a fresh slot with a key she holds; a claimant drips
/// once, alone, exactly the posted drip, and marks her own flag; nothing
/// changes the config.
const FAUCET: &str = r#"
model Faucet {
  part flow {
    q0 --> q1
    q1 --> q1: +POST -SEND -CREATE -modifies(/claimants) -modifies(/config)
    q1 --> q1: +POST -SEND -CREATE -modifies(/config) -state_exists(/claimants/$k.id) +post_to_path(/claimants/$k.id) +posts_own_key(/claimants/$k.id) -modifies(/claimants/$k) -modifies(/claimants/!$k)
    q1 --> q1: +SEND +POST -CREATE -modifies(/config) +signed_by(/claimants/$k.id) -signed_by(/claimants/!$k.id) -bool_true(/claimants/$k/claimed.bool) +post_to(/claimants/$k/claimed.bool, "true") -modifies(/claimants/$k.id) -modifies(/claimants/!$k) +sent_eq("drops", /config/drip.num)
  }
}
"#;

const FAUCET_RULES: &[&str] = &[
    // The pool is finite, and the drip size is fixed.
    "always([+CREATE] false)",
    "always([+modifies(/config)] false)",
    // Every SEND is exactly the posted drip, signed by a claimant.
    r#"always([+SEND -sent_eq("drops", /config/drip.num)] false)"#,
    "always([+SEND -any_signed(/claimants)] false)",
    // A key is registered by its holder; a slot is written by its key.
    "always([+post_to_path(/claimants/$k.id) -posts_own_key(/claimants/$k.id)] false)",
    "always([+modifies(/claimants/$k) -signed_by(/claimants/$k.id)] false)",
    "always([+modifies(/claimants/$k.id) +state_exists(/claimants/$k.id) -signed_by(/claimants/$k.id)] false)",
    // Each claimant drips once and marks her flag.
    "always([+SEND +signed_by(/claimants/$k.id) +bool_true(/claimants/$k/claimed.bool)] false)",
    r#"always([+SEND +signed_by(/claimants/$k.id) -post_to(/claimants/$k/claimed.bool, "true")] false)"#,
];

fn faucet_bootstrap(model: &str) -> CommitFile {
    let mut c = commit(vec![
        ("model", "/model/default.modality", json!(model)),
        ("post", "/config/drip.num", json!(10)),
        ("post", "/claimants/alice.id", json!("KEY_A")),
        ("post", "/claimants/bob.id", json!("KEY_B")),
    ]);
    c.add_action(
        "create".to_string(),
        None,
        json!({"asset_id": "drops", "quantity": 1000, "divisibility": 1}),
    );
    c
}

fn drip(claimant: &str, amounts: &[u64], key: &str) -> CommitFile {
    let mut c = sends(
        &amounts
            .iter()
            .map(|a| ("drops", "wallet", json!(a)))
            .collect::<Vec<_>>(),
    );
    c.add_action(
        "post".to_string(),
        Some(format!("/claimants/{claimant}/claimed.bool")),
        json!(true),
    );
    signed(c, &[key])
}

fn register(slot: &str, key: &str, signer: &str) -> CommitFile {
    signed(
        commit(vec![("post", &format!("/claimants/{slot}.id"), json!(key))]),
        &[signer],
    )
}

fn faucet_with_rules() -> Vec<CommitFile> {
    let mut accepted = vec![faucet_bootstrap(FAUCET)];
    for rule in FAUCET_RULES {
        let pending = rule_commit(rule);
        validate(&accepted, &pending, V2).unwrap_or_else(|e| panic!("{rule}: {e}"));
        accepted.push(pending);
    }
    accepted
}

#[test]
fn faucet_drips_the_posted_amount_once_to_registered_holders() {
    // `V0` reads labels as names: it cannot see that a claimant's key is a
    // key under `/claimants`, or that `-modifies(/claimants)` rules out a post
    // into a slot. `V2` can. Hence `modal commit --theory v2`.
    for rule in [FAUCET_RULES[3], FAUCET_RULES[4]] {
        validate(&[faucet_bootstrap(FAUCET)], &rule_commit(rule), V0).expect_err(rule);
    }
    let mut accepted = faucet_with_rules();
    let view = derived_view("", &accepted, TheoryActivation::always(V2)).unwrap();
    assert!(view.dead_edges.is_empty(), "{:?}", view.dead_edges);

    // A stranger registers her own key; nobody registers a key they do not hold.
    let err = validate(&accepted, &register("dave", "KEY_D", "KEY_M"), V2)
        .expect_err("a key its holder did not post");
    assert!(err.to_string().contains("posts_own_key"), "{err}");
    let carol = register("carol", "KEY_C", "KEY_C");
    // A registration writes its slot and nothing else under the registry.
    for (path, why) in [
        ("/claimants/notes.text", "a file beside the slots"),
        ("/claimants/eve.id", "a second slot"),
        ("/claimants/alice/claimed.bool", "another claimant's flag"),
    ] {
        let mut extra = carol.clone();
        extra.add_action("post".to_string(), Some(path.to_string()), json!("KEY_C"));
        for theory in [V0, V2] {
            validate(&accepted, &extra, theory).expect_err(why);
        }
    }
    // The registry path itself has no type, so no commit may write it.
    let mut untyped = carol.clone();
    untyped.add_action("post".to_string(), Some("/claimants".to_string()), json!("KEY_C"));
    assert!(untyped.validate().is_err());
    // A sibling of the registry is no slot: no claimant reads it.
    let mut sibling = carol.clone();
    sibling.add_action("post".to_string(), Some("/claimants.id".to_string()), json!("KEY_C"));
    validate(&accepted, &sibling, V2).expect("outside the registry");
    validate(&accepted, &carol, V2).expect("Carol registers herself");
    accepted.push(carol);

    // The drip is exactly the posted amount, however it is split.
    for (amounts, why) in [(&[11][..], "too much"), (&[9][..], "too little"), (&[10, 1][..], "an extra SEND")] {
        for theory in [V0, V2] {
            let err = validate(&accepted, &drip("carol", amounts, "KEY_C"), theory).expect_err(why);
            assert!(err.to_string().contains("sent_eq"), "{why}: {err}");
        }
    }
    validate(&accepted, &drip("carol", &[4, 6], "KEY_C"), V2).expect("ten in two SENDs");
    // A drip that writes its flag false would leave the next drip open.
    let mut unmarked = drip("carol", &[10], "KEY_C");
    unmarked.body.last_mut().unwrap().value = json!(false);
    for theory in [V0, V2] {
        let err = validate(&accepted, &unmarked, theory).expect_err("the flag must be set");
        assert!(err.to_string().contains("post_to"), "{err}");
    }
    let first = drip("carol", &[10], "KEY_C");
    validate(&accepted, &first, V2).expect("Carol's drip");
    accepted.push(first);
    let err = validate(&accepted, &drip("carol", &[10], "KEY_C"), V2).expect_err("second drip");
    assert!(err.to_string().contains("claimed.bool"), "{err}");

    // The config does not move, so neither does the drip size.
    let bump = signed(commit(vec![("post", "/config/drip.num", json!(1000))]), &["KEY_A"]);
    validate(&accepted, &bump, V2).expect_err("the drip size is fixed");

    // The rules outlive the model: a replacement that drops the amount or
    // the key check is refused.
    for (from, to) in [
        (r#" +sent_eq("drops", /config/drip.num)"#, ""),
        (" +posts_own_key(/claimants/$k.id)", ""),
    ] {
        let sloppy = FAUCET.replace(from, to);
        let replace = commit(vec![("model", "/model/default.modality", json!(sloppy))]);
        let err = validate(&accepted, &replace, V2).expect_err(from);
        assert!(err.to_string().contains("Model violates rule"), "{from}: {err}");
    }
}

// ---------------------------------------------------------------------------
// Outflow only through a posted program
// ---------------------------------------------------------------------------

const PAYOUT: &str = "/__programs__/payout.wasm";
const PAYOUT_SHA: &str = "5f1e";

/// Only the payout program's output moves assets; its bytes do not change.
const TREASURY: &str = r#"
model Treasury {
  part flow {
    q0 --> q1
    q1 --> q1: +POST -SEND -CREATE -modifies(/__programs__)
    q1 --> q1: +SEND -CREATE -modifies(/__programs__) +emitted_by(/__programs__/payout.wasm, "5f1e")
  }
}
"#;

#[test]
fn only_the_posted_program_moves_the_treasury() {
    let bootstrap = commit(vec![
        ("model", "/model/default.modality", json!(TREASURY)),
        ("post", "/owner.id", json!("KEY_O")),
    ]);
    let mut accepted = vec![bootstrap];
    for rule in [
        r#"always([+SEND -emitted_by(/__programs__/payout.wasm, "5f1e")] false)"#,
        "always([+modifies(/__programs__)] false)",
    ] {
        let pending = rule_commit(rule);
        validate(&accepted, &pending, V2).unwrap_or_else(|e| panic!("{rule}: {e}"));
        accepted.push(pending);
    }

    let payout = || sends(&[("drops", "w1", json!(5))]);
    validate(&accepted, &emitted(payout(), PAYOUT, PAYOUT_SHA), V2).expect("the program pays");
    for (pending, why) in [
        (signed(payout(), &["KEY_O"]), "the owner writes a SEND by hand"),
        (emitted(payout(), "/__programs__/other.wasm", PAYOUT_SHA), "another program"),
        (emitted(payout(), PAYOUT, "0000"), "other bytes at the same path"),
    ] {
        for theory in [V0, V2] {
            let err = validate(&accepted, &pending, theory).expect_err(why);
            assert!(err.to_string().contains("emitted_by"), "{why}: {err}");
        }
    }
    let mut mixed = emitted(payout(), PAYOUT, PAYOUT_SHA);
    mixed.add_action("send".to_string(), None, json!({"asset_id": "drops", "to_contract": "me", "amount": 500}));
    validate(&accepted, &mixed, V2).expect_err("a hand-written SEND rides along");

    // Rewriting the program is refused, and so is a model that lets the owner pay.
    let swap = commit(vec![("post", PAYOUT, json!("AA=="))]);
    validate(&accepted, &swap, V2).expect_err("the program is fixed");
    let owner_pays = TREASURY.replace(
        r#"+emitted_by(/__programs__/payout.wasm, "5f1e")"#,
        "+signed_by(/owner.id)",
    );
    let replace = commit(vec![("model", "/model/default.modality", json!(owner_pays))]);
    let err = validate(&accepted, &replace, V2).expect_err("rules still bind");
    assert!(err.to_string().contains("Model violates rule"), "{err}");
}


// ---------------------------------------------------------------------------
// A pool's swap invariant: the reserve product does not fall
// ---------------------------------------------------------------------------

fn reserves(a: Value, b: Value) -> CommitFile {
    commit(vec![("post", "/reserves/a.num", a), ("post", "/reserves/b.num", b)])
}

#[test]
fn keeps_product_compares_the_product_after_the_commit_with_before() {
    let state = [
        ("/reserves/a.num", json!(100)),
        ("/reserves/b.num", json!(100)),
        ("/config/fee.num", json!(0.003)),
        ("/config/fee.text", json!("0.003")),
        ("/reserves/neg.num", json!(-1)),
    ];
    let k = |c: &CommitFile, args: &[&str]| holds(c, &state, "keeps_product", args);
    let ab = ["/reserves/a.num", "/reserves/b.num"];
    let with_fee = |fee: &'static str| ["/reserves/a.num", "/reserves/b.num", fee];

    // 110 * 91 = 10010 and 110 * 90.9 = 9999, against 100 * 100.
    assert!(k(&reserves(json!(110), json!(91)), &ab));
    assert!(!k(&reserves(json!(110), json!(90.9)), &ab));
    assert!(k(&CommitFile::new(), &ab), "unchanged reserves keep the product");
    // Only 99.7% of what goes in counts: 109.97 * 90.91 < 10000 <= 110 * 90.91.
    let no_fee_paid = reserves(json!(110), json!(90.91));
    assert!(k(&no_fee_paid, &ab));
    assert!(!k(&no_fee_paid, &with_fee("0.003")));
    assert!(!k(&no_fee_paid, &with_fee("/config/fee.num")));
    assert!(k(&reserves(json!(110), json!(91)), &with_fee("/config/fee.num")));
    // A commit that writes one reserve twice ends at the last write.
    let mut twice = reserves(json!(50), json!(91));
    twice.add_action("post".to_string(), Some("/reserves/a.num".to_string()), json!(110));
    assert!(k(&twice, &ab));

    let fine = reserves(json!(110), json!(91));
    for (args, why) in [
        (vec!["/reserves/a.num", "/reserves/a.num"], "one path twice"),
        (vec!["/reserves/a.text", "/reserves/b.num"], "not a .num path"),
        (vec!["/reserves/c.num", "/reserves/b.num"], "no accepted number"),
        (vec!["/reserves/neg.num", "/reserves/b.num"], "a negative reserve"),
        (with_fee("1").to_vec(), "a fee of one"),
        (with_fee("-0.1").to_vec(), "a negative fee"),
        (with_fee("/config/fee.text").to_vec(), "a fee path of another type"),
        (with_fee("0.3%").to_vec(), "a fee that is not a decimal"),
        (vec!["/reserves/a.num", "/reserves/b.num", "0", "extra"], "too many arguments"),
    ] {
        assert!(!k(&fine, &args), "{why}");
    }
    assert!(!k(&reserves(json!("110"), json!(91)), &ab), "a pending write that is text");
    assert!(!k(&reserves(json!(-5), json!(-3000)), &ab), "a pending negative reserve");
}

const POOL_PROGRAM: &str = "/__programs__/pool.wasm";

/// Only the pool program writes the reserves, a swap keeps the fee-adjusted
/// product, and neither the config nor the program changes.
const POOL: &str = r#"
model Pool {
  part flow {
    q0 --> q1
    q1 --> q1: +POST -CREATE -modifies(/reserves) -modifies(/config) -modifies(/__programs__)
    q1 --> q1: +POST -CREATE -modifies(/config) -modifies(/__programs__) +emitted_by(/__programs__/pool.wasm, "5f1e") +keeps_product(/reserves/a.num, /reserves/b.num, /config/fee.num)
  }
}
"#;

#[test]
fn a_pool_program_that_drains_or_skips_the_fee_is_refused() {
    let mut bootstrap = reserves(json!(100), json!(100));
    bootstrap.add_action("model".to_string(), Some("/model/default.modality".to_string()), json!(POOL));
    bootstrap.add_action("post".to_string(), Some("/config/fee.num".to_string()), json!(0.003));
    let mut accepted = vec![bootstrap];
    for rule in [
        r#"always([+modifies(/reserves) -emitted_by(/__programs__/pool.wasm, "5f1e")] false)"#,
        "always([+modifies(/reserves) -keeps_product(/reserves/a.num, /reserves/b.num, /config/fee.num)] false)",
        "always([+modifies(/config)] false)",
        "always([+modifies(/__programs__)] false)",
    ] {
        let pending = rule_commit(rule);
        validate(&accepted, &pending, V2).unwrap_or_else(|e| panic!("{rule}: {e}"));
        accepted.push(pending);
    }

    let swap = |a: Value, b: Value| emitted(reserves(a, b), POOL_PROGRAM, PAYOUT_SHA);
    for (pending, why) in [
        (swap(json!(110), json!(50)), "a program that drains b"),
        (swap(json!(110), json!(90.91)), "a program that skips the fee"),
    ] {
        for theory in [V0, V2] {
            let err = validate(&accepted, &pending, theory).expect_err(why);
            assert!(err.to_string().contains("keeps_product"), "{why}: {err}");
        }
    }
    let good = swap(json!(110), json!(91));
    validate(&accepted, &good, V2).expect("a swap that pays the fee");
    accepted.push(good);
    // The next swap is judged against the reserves the last one left.
    validate(&accepted, &swap(json!(100), json!(100)), V2)
        .expect_err("swapping back at the old price loses the fee");

    // A model that drops the invariant is refused: the rule outlives it.
    let no_invariant = POOL.replace(
        " +keeps_product(/reserves/a.num, /reserves/b.num, /config/fee.num)",
        "",
    );
    let replace = commit(vec![("model", "/model/default.modality", json!(no_invariant))]);
    let err = validate(&accepted, &replace, V2).expect_err("rules still bind");
    assert!(err.to_string().contains("Model violates rule"), "{err}");
}

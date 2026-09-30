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
const V3: TheoryVersion = TheoryVersion::V3;

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

fn holds_under(theory: TheoryVersion, state: &[(&str, Value)], name: &str, args: &[&str]) -> bool {
    let state: HashMap<String, Value> = state
        .iter()
        .map(|(k, v)| (k.trim_start_matches('/').to_string(), v.clone()))
        .collect();
    CommitFacts::from_commit(&CommitFile::new(), &state)
        .under(theory)
        .predicate_holds(&Property::new_predicate_from_call_args(
            name.to_string(),
            args.iter().map(|a| a.to_string()).collect(),
        ))
}

#[test]
fn v3_compares_numbers_exactly_where_f64_rounds() {
    let state = [
        ("/big.num", json!(9_007_199_254_740_993u64)),
        ("/tenth.num", json!(0.1)),
    ];
    let under = |theory, name, args: &[&str]| holds_under(theory, &state, name, args);
    let two53 = "9007199254740992";
    assert!(!under(V2, "num_gt", &["/big.num", two53]), "f64 rounds 2^53 + 1 down");
    assert!(under(V2, "num_eq", &["/big.num", two53]));
    assert!(under(V3, "num_gt", &["/big.num", two53]));
    assert!(!under(V3, "num_eq", &["/big.num", two53]));
    assert!(under(V3, "num_eq", &["/big.num", "9007199254740993"]));
    assert!(!under(V3, "num_lte", &["/big.num", "9.007199254740993e15"]), "no exponent literals");

    let near = "0.10000000000000001";
    assert!(under(V2, "num_eq", &["/tenth.num", near]), "one f64 for both");
    assert!(under(V3, "num_lt", &["/tenth.num", near]));
    assert!(under(V3, "num_eq", &["/tenth.num", "0.1"]));
    assert!(under(V3, "num_gte", &["/big.num", "/tenth.num"]), "a path on the right");

    let range = |theory, lo: &str, hi: &str| under(theory, "amount_in_range", &["/big.num", lo, hi]);
    assert!(range(V2, two53, two53), "f64 rounds 2^53 + 1 into [2^53, 2^53]");
    assert!(!range(V3, two53, two53));
    assert!(range(V3, two53, "9007199254740994"));
    assert!(!under(V3, "num_gt", &["/big.num", "many"]), "not a number");
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

// ---------------------------------------------------------------------------
// The full pool: reserves track what moves, payouts go back to who paid in,
// swaps keep the fee-adjusted product, liquidity moves keep it per share
// ---------------------------------------------------------------------------

const TOK_A: &str = "KA:tokA";
const TOK_B: &str = "KB:tokB";

fn act(method: &str, path: Option<&str>, value: Value) -> CommitAction {
    let mut c = CommitFile::new();
    c.add_action(method.to_string(), path.map(str::to_string), value);
    c.body.pop().unwrap()
}

fn held(asset: &str) -> (Option<&str>, &str) {
    match asset.split_once(':') {
        Some((creator, id)) => (Some(creator), id),
        None => (None, asset),
    }
}

fn recv(from: &str, asset: &str, amount: u64) -> CommitAction {
    let (creator, id) = held(asset);
    let mut value = json!({"send_commit_id": "s", "from_contract": from, "asset_id": id, "amount": amount});
    if let Some(creator) = creator {
        value["asset_contract"] = json!(creator);
    }
    act("recv", None, value)
}

fn recv_memo(from: &str, asset: &str, amount: u64, memo: Value) -> CommitAction {
    let mut action = recv(from, asset, amount);
    action.value["memo"] = memo;
    action
}

fn send(asset: &str, to: &str, amount: u64) -> CommitAction {
    let (creator, id) = held(asset);
    let mut value = json!({"asset_id": id, "to_contract": to, "amount": amount});
    if let Some(creator) = creator {
        value["asset_contract"] = json!(creator);
    }
    act("send", None, value)
}

fn post(path: &str, value: Value) -> CommitAction {
    act("post", Some(path), value)
}

fn pool_output(actions: Vec<CommitAction>) -> CommitFile {
    let mut c = CommitFile::new();
    c.body = actions;
    emitted(c, POOL_PROGRAM, PAYOUT_SHA)
}

fn reserves_after(a: u64, b: u64) -> Vec<CommitAction> {
    vec![post("/reserves/a.num", json!(a)), post("/reserves/b.num", json!(b))]
}

fn tracks_all() -> String {
    format!(
        r#"+tracks(/reserves/a.num, "{TOK_A}") +tracks(/reserves/b.num, "{TOK_B}") +tracks(/lp/supply.num, "lp", "issued") +keeps_product_per_share(/reserves/a.num, /reserves/b.num, /lp/supply.num, /config/fee.num) +pays_senders("{TOK_A}") +pays_senders("{TOK_B}") +pays_senders("lp") +pays_memo_min("min_out") +pays_memo_min("min_shares")"#
    )
}

fn full_pool_model() -> String {
    let t = tracks_all();
    let paid = r#"+emitted_by(/__programs__/pool.wasm, "5f1e") -CREATE -modifies(/config) -modifies(/__programs__)"#;
    format!(
        "model Pool {{\n  part flow {{\n    q0 --> q1\n    q1 --> q1: -SEND -RECV -CREATE -modifies(/reserves) -modifies(/lp) -modifies(/config) -modifies(/__programs__) {t}\n    q1 --> q1: {paid} {t} +keeps_product(/reserves/a.num, /reserves/b.num, /config/fee.num)\n    q1 --> q1: {paid} {t} +modifies(/lp)\n  }}\n}}\n"
    )
}

fn full_pool_rules() -> Vec<String> {
    let mut rules = vec![
        r#"always([+SEND -emitted_by(/__programs__/pool.wasm, "5f1e")] false)"#.to_string(),
        r#"always([+RECV -emitted_by(/__programs__/pool.wasm, "5f1e")] false)"#.to_string(),
    ];
    for asset in [TOK_A, TOK_B, "lp"] {
        rules.push(format!(r#"always([-pays_senders("{asset}")] false)"#));
    }
    rules.extend([
        format!(r#"always([-tracks(/reserves/a.num, "{TOK_A}")] false)"#),
        format!(r#"always([-tracks(/reserves/b.num, "{TOK_B}")] false)"#),
        r#"always([-tracks(/lp/supply.num, "lp", "issued")] false)"#.to_string(),
        "always([+modifies(/reserves) -modifies(/lp) -keeps_product(/reserves/a.num, /reserves/b.num, /config/fee.num)] false)".to_string(),
        "always([-keeps_product_per_share(/reserves/a.num, /reserves/b.num, /lp/supply.num, /config/fee.num)] false)".to_string(),
        r#"always([-pays_memo_min("min_out")] false)"#.to_string(),
        r#"always([-pays_memo_min("min_shares")] false)"#.to_string(),
        "always([+modifies(/config)] false)".to_string(),
        "always([+modifies(/__programs__)] false)".to_string(),
        "always([+CREATE] false)".to_string(),
    ]);
    rules
}

#[test]
fn tracks_and_pays_senders_read_what_the_commit_moves() {
    let state = [("/reserves/a.num", json!(100)), ("/lp/supply.num", json!(10))];
    let h = |c: &CommitFile, name: &str, args: &[&str]| holds(c, &state, name, args);
    let c = |actions: Vec<CommitAction>| {
        let mut c = CommitFile::new();
        c.body = actions;
        c
    };

    let swap_in = c(vec![recv("T", TOK_A, 7), post("/reserves/a.num", json!(107))]);
    assert!(h(&swap_in, "tracks", &["/reserves/a.num", TOK_A]));
    assert!(!h(&swap_in, "tracks", &["/reserves/a.num", "tokA"]), "an own asset of that id is another asset");
    let pay_out = c(vec![send(TOK_A, "T", 7), post("/reserves/a.num", json!(93))]);
    assert!(h(&pay_out, "tracks", &["/reserves/a.num", TOK_A]));
    let skim = c(vec![send(TOK_A, "T", 7), post("/reserves/a.num", json!(95))]);
    assert!(!h(&skim, "tracks", &["/reserves/a.num", TOK_A]), "the reserve must fall by what left");
    assert!(!h(&c(vec![send(TOK_A, "T", 7)]), "tracks", &["/reserves/a.num", TOK_A]), "unposted outflow");
    assert!(h(&c(vec![]), "tracks", &["/reserves/a.num", TOK_A]), "nothing moves, nothing changes");
    let mut unstated = recv("T", TOK_A, 7);
    unstated.value.as_object_mut().unwrap().remove("amount");
    assert!(!h(&c(vec![unstated, post("/reserves/a.num", json!(107))]), "tracks", &["/reserves/a.num", TOK_A]));

    let mint = c(vec![send("lp", "L", 5), post("/lp/supply.num", json!(15))]);
    assert!(h(&mint, "tracks", &["/lp/supply.num", "lp", "issued"]));
    assert!(!h(&mint, "tracks", &["/lp/supply.num", "lp"]), "a supply grows when shares go out");
    assert!(!h(&mint, "tracks", &["/lp/supply.num", "lp", "held"]));

    assert!(h(&c(vec![recv("T", TOK_A, 7), send(TOK_B, "T", 3)]), "pays_senders", &[TOK_B]));
    assert!(!h(&c(vec![recv("T", TOK_A, 7), send(TOK_B, "M", 3)]), "pays_senders", &[TOK_B]));
    assert!(h(&c(vec![recv("T", TOK_A, 7), send(TOK_B, "M", 3)]), "pays_senders", &[TOK_A]), "no SEND of it");
    assert!(!h(&c(vec![send(TOK_B, "T", 3)]), "pays_senders", &[TOK_B]), "nobody paid in");

    // sent_* name a held asset by its creator, and an own asset by its id.
    let both = c(vec![send(TOK_A, "T", 4), send("tokA", "T", 6)]);
    assert!(h(&both, "sent_eq", &[TOK_A, "4"]));
    assert!(h(&both, "sent_eq", &["tokA", "6"]));

    // Liquidity per share: 100 * 100 * 10^2 against 150 * 150 * 15^2 after a proportional add.
    let per = |a: u64, s: u64| {
        let mut commit = c(vec![]);
        commit.body = vec![post("/reserves/a.num", json!(a)), post("/reserves/b.num", json!(a)), post("/lp/supply.num", json!(s))];
        holds(&commit, &[("/reserves/a.num", json!(100)), ("/reserves/b.num", json!(100)), ("/lp/supply.num", json!(10))], "keeps_product_per_share", &["/reserves/a.num", "/reserves/b.num", "/lp/supply.num"])
    };
    assert!(per(150, 15), "a proportional add");
    assert!(!per(150, 16), "one share too many");
    assert!(per(50, 5), "a pro-rata remove");
    assert!(!per(49, 5), "paid out too much");

    // With a fee, an unchanged supply holds a swap to the fee-adjusted product.
    let fee_state = [("/reserves/a.num", json!(1000)), ("/reserves/b.num", json!(4000)), ("/lp/supply.num", json!(2000)), ("/config/fee.num", json!(0.003))];
    let swap_writing_supply = |out: u64| {
        c(vec![post("/reserves/a.num", json!(1100)), post("/reserves/b.num", json!(4000 - out)), post("/lp/supply.num", json!(2000))])
    };
    let args = ["/reserves/a.num", "/reserves/b.num", "/lp/supply.num", "/config/fee.num"];
    assert!(holds(&swap_writing_supply(363), &fee_state, "keeps_product_per_share", &args[..3]), "no fee asked");
    assert!(!holds(&swap_writing_supply(363), &fee_state, "keeps_product_per_share", &args), "skips the fee");
    assert!(holds(&swap_writing_supply(362), &fee_state, "keeps_product_per_share", &args), "pays it");

    // A memo's minimum: paid at least that of another asset, or refunded.
    let min_out = |n: u64| json!({"op": "swap", "min_out": n});
    let asked = |memo: Value, answer: Vec<CommitAction>| {
        let mut actions = vec![recv_memo("T", TOK_A, 100, memo)];
        actions.extend(answer);
        h(&c(actions), "pays_memo_min", &["min_out"])
    };
    assert!(asked(min_out(300), vec![send(TOK_B, "T", 362)]));
    assert!(!asked(min_out(400), vec![send(TOK_B, "T", 362)]), "below min_out");
    assert!(asked(min_out(400), vec![send(TOK_A, "T", 100)]), "the deposit returned");
    assert!(!asked(min_out(400), vec![send(TOK_A, "T", 99)]), "not all of it");
    assert!(!asked(min_out(300), vec![send(TOK_B, "M", 362)]), "paid to someone else");
    assert!(asked(json!("{\"op\":\"swap\",\"min_out\":300}"), vec![send(TOK_B, "T", 362)]), "a memo as JSON text");
    assert!(asked(json!({"op": "remove"}), vec![]), "no min_out asked");
    assert!(h(&c(vec![recv("T", TOK_A, 100)]), "pays_memo_min", &["min_out"]), "no memo");
    let mut unstated = recv_memo("T", TOK_A, 100, min_out(300));
    unstated.value.as_object_mut().unwrap().remove("from_contract");
    assert!(!h(&c(vec![unstated, send(TOK_B, "T", 362)]), "pays_memo_min", &["min_out"]));
}

#[test]
fn a_pool_pays_only_who_paid_in_and_keeps_its_product() {
    let mut bootstrap = commit(vec![
        ("model", "/model/default.modality", json!(full_pool_model())),
        ("post", "/config/fee.num", json!(0.003)),
        ("post", "/reserves/a.num", json!(0)),
        ("post", "/reserves/b.num", json!(0)),
        ("post", "/lp/supply.num", json!(0)),
    ]);
    bootstrap.add_action("create".to_string(), None, json!({"asset_id": "lp", "quantity": 1_000_000_000u64, "divisibility": 1}));
    let mut accepted = vec![bootstrap];
    for rule in full_pool_rules() {
        let pending = rule_commit(&rule);
        validate(&accepted, &pending, V2).unwrap_or_else(|e| panic!("{rule}: {e}"));
        accepted.push(pending);
    }

    // L adds 1000 A and 4000 B; the first deposit mints sqrt(1000 * 4000) shares.
    let add = pool_output(
        [vec![recv("L", TOK_A, 1000), recv("L", TOK_B, 4000), send("lp", "L", 2000)], reserves_after(1000, 4000), vec![post("/lp/supply.num", json!(2000))]].concat(),
    );
    validate(&accepted, &add, V2).expect("the first deposit");
    accepted.push(add);

    // T swaps 100 A: out = 4000 * 100 * 997 / (1000 * 1000 + 100 * 997) = 362.6.
    let swap = |out: u64, to: &str, a_after: u64| {
        pool_output([vec![recv("T", TOK_A, 100), send(TOK_B, to, out)], reserves_after(a_after, 4000 - out)].concat())
    };
    for (pending, why, says) in [
        (swap(400, "T", 1100), "pays more than the curve", "keeps_product"),
        (swap(362, "M", 1100), "pays someone else", "pays_senders"),
        (swap(362, "T", 1200), "posts reserves the flow does not explain", "tracks"),
    ] {
        for theory in [V0, V2] {
            let err = validate(&accepted, &pending, theory).expect_err(why);
            assert!(err.to_string().contains(says), "{why}: {err}");
        }
    }
    let mut by_hand = swap(362, "T", 1100);
    by_hand.body.iter_mut().for_each(|a| a.emitted_by = None);
    validate(&accepted, &by_hand, V2).expect_err("written by hand");
    // A swap that also writes the share supply, unchanged, still pays the fee.
    let mut no_fee = swap(363, "T", 1100);
    no_fee.body.push(post("/lp/supply.num", json!(2000)));
    for theory in [V0, V2] {
        let err = validate(&accepted, &no_fee, theory).expect_err("skips the fee");
        assert!(err.to_string().contains("keeps_product_per_share"), "{err}");
    }
    // A swap that pays less than the trader's min_out, rather than refunding.
    let short = |memo_min: u64| {
        let memo = json!({"op": "swap", "min_out": memo_min});
        pool_output([vec![recv_memo("T", TOK_A, 100, memo), send(TOK_B, "T", 362)], reserves_after(1100, 3638)].concat())
    };
    let err = validate(&accepted, &short(400), V2).expect_err("below min_out");
    assert!(err.to_string().contains("pays_memo_min"), "{err}");
    validate(&accepted, &short(300), V2).expect("meets min_out");
    let refund = pool_output(vec![
        recv_memo("T", TOK_A, 100, json!({"op": "swap", "min_out": 400})),
        send(TOK_A, "T", 100),
    ]);
    validate(&accepted, &refund, V2).expect("the deposit returned");

    let good = swap(362, "T", 1100);
    validate(&accepted, &good, V2).expect("a swap on the curve");
    accepted.push(good);

    // L returns 1000 of 2000 shares: half of each reserve, rounded down.
    let remove = |a_out: u64, b_out: u64, burned_to: u64| {
        pool_output(
            [
                vec![recv("L", "lp", 1000), send(TOK_A, "L", a_out), send(TOK_B, "L", b_out)],
                reserves_after(1100 - a_out, 3638 - b_out),
                vec![post("/lp/supply.num", json!(burned_to))],
            ]
            .concat(),
        )
    };
    let err = validate(&accepted, &remove(600, 1819, 1000), V2).expect_err("more than half of A");
    assert!(err.to_string().contains("keeps_product_per_share"), "{err}");
    let err = validate(&accepted, &remove(550, 1819, 1100), V2).expect_err("the supply falls by what came back");
    assert!(err.to_string().contains("tracks"), "{err}");
    validate(&accepted, &remove(550, 1819, 1000), V2).expect("a pro-rata remove");

    // The rules outlive the model.
    let lax = full_pool_model().replace(&format!(r#" +pays_senders("{TOK_B}")"#), "");
    let replace = commit(vec![("model", "/model/default.modality", json!(lax))]);
    let err = validate(&accepted, &replace, V2).expect_err("rules still bind");
    assert!(err.to_string().contains("Model violates rule"), "{err}");
}

/// A header at `index` after `previous`, paying `to`, mined with sha256 at
/// difficulty 1. Its hash, as the miner names blocks, is RandomX.
fn mined(index: u64, previous: &str, to: &str) -> Value {
    let data_hash = crate::miner_header::data_hash(to, 7);
    let data = crate::miner_header::mining_data(index, 1_700_000_000, previous, &data_hash, 1);
    let nonce = crate::hash_tax::mine(&data, 1, None, Some("sha256")).unwrap();
    let hash = crate::hash_tax::hash_with_nonce(&data, nonce, "randomx").unwrap();
    json!({
        "index": index, "to": to, "hash": hash, "previous_hash": previous,
        "timestamp": 1_700_000_000i64, "data_hash": data_hash,
        "difficulty": "1", "nonce": nonce.to_string(), "miner_number": 7,
    })
}

fn posting(headers: &[(u64, &Value)]) -> CommitFile {
    let mut c = CommitFile::new();
    for (index, header) in headers {
        c.add_action(
            "post".to_string(),
            Some(format!("/emission/blocks/{index}.json")),
            (*header).clone(),
        );
    }
    c
}

#[test]
fn mined_headers_holds_only_for_linked_headers_with_their_work() {
    let sha = ("/network/emission/hash_func.text", json!("sha256"));
    let genesis = ("/network/emission/genesis_block_hash.text", json!("g0"));
    let b1 = mined(1, "g0", "12D3KooWAlice");
    let b2 = mined(2, b1["hash"].as_str().unwrap(), "12D3KooWBob");
    let check = |c: &CommitFile, state: &[(&str, Value)]| {
        holds(c, state, "mined_headers", &["/emission/blocks"])
    };

    assert!(check(&posting(&[(1, &b1), (2, &b2)]), &[sha.clone(), genesis.clone()]));
    assert!(check(&CommitFile::new(), &[sha.clone()]), "no header posted");
    assert!(
        check(&posting(&[(2, &b2)]), &[sha.clone(), ("/emission/blocks/1.json", b1.clone())]),
        "block 2 links to an accepted block 1"
    );
    assert!(!check(&posting(&[(2, &b2)]), &[sha.clone()]), "block 1 is nowhere");
    assert!(
        !check(&posting(&[(1, &b1)]), &[sha.clone(), ("/network/emission/genesis_block_hash.text", json!("g1"))]),
        "block 1 must follow the genesis block"
    );
    assert_eq!(
        check(&posting(&[(1, &b1)]), &[genesis.clone()]),
        crate::hash_tax::is_hash_acceptable(b1["hash"].as_str().unwrap(), 1, "randomx"),
        "the default proof of work is randomx: the hash itself must meet the difficulty"
    );

    let tampered = |field: &str, value: Value| {
        let mut h = b1.clone();
        h[field] = value;
        check(&posting(&[(1, &h)]), &[sha.clone(), genesis.clone()])
    };
    assert!(!tampered("to", json!("12D3KooWMallory")), "the payee is under the data hash");
    assert!(!tampered("miner_number", json!(8)));
    assert!(!tampered("nonce", json!("0")) || b1["nonce"] == json!("0"));
    assert!(!tampered("hash", json!("00")));
    assert!(!tampered("index", json!(3)), "the header's index is its path's");
    assert!(!tampered("difficulty", json!("1000000000000")), "the hash does not meet it");
    assert!(!check(&posting(&[(0, &b1)]), &[sha]), "block 0 is genesis, not mined");
}

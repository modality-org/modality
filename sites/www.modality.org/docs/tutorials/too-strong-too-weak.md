---
sidebar_position: 10
title: Too Strong, Then Too Weak
---

# Too Strong, Then Too Weak

The membership contract in [A move that never happens](members-only-contract)
asks for two things: any member may post a note, and changing membership
takes every member. A rule can verify and still miss that request. Too
strong locks out the note. Too weak lets one member edit the list. This
page causes both, then lands the commit the request actually allowed.

## Too strong

Any member may write, as long as they do not change `/members`. That witness
is the one the request allows. Pair it with a rule that demands every
member on every later commit.

```bash
modal contract create --dir ./judgment
cd judgment
modal checkout
modal set-named-id /members/carol.id example/alice
modal set-named-id /members/dave.id example/bob
modal set-named-id /members/erin.id example/erin
modal add-rule --name signed 'always([-any_signed(/members)] false)'
modal add-rule --name too_strong 'always([-all_signed(/members)] false)'
```

```modality
model Contract {
  part flow {
    q0 --> q1
    q1 --> q1: +any_signed(/members) -modifies(/members)
    q1 --> q1: +any_signed(/members) +modifies(/members) +all_signed(/members)
  }
}
```

```bash
modal commit --all -m "Too strong"
```

```output
Model violates rule 'local_rule'
formula: always([-all_signed(/members)] false)
counterexample: box [-all_signed(/members)] false failed because matching transition targets violated it: q1 -> q1 [+any_signed(/members) -modifies(/members)]
```

The note edge is a step one member can take. The too-strong rule says that
step is forbidden. The checker refuses the witness, so the parties are not
stuck with a rule that locks the note. Delete `rules/too_strong.modality`.

## The request, as a rule

```bash
modal add-rule --name membership \
  'always([+modifies(/members) -all_signed(/members)] false)'
modal commit --all -m "Members"
modal commit --path /notes.text --value hello --sign example/alice -m "Carol posts a note"
modal set-named-id /members/frank.id example/frank
modal commit --all --sign example/alice -m "Carol adds Frank"
```

The note lands. Carol adding Frank is refused, the same way as in the
membership page: the note edge forbids a change to `/members`, and the
membership edge wants every member. All three sign, and Frank is added.

```bash
modal commit --all --sign example/alice --sign example/bob --sign example/erin -m "All add Frank"
```

## Too weak

A fresh directory, with the formula that only asks for some member's
signature on a membership change. The witness lets any signed step through,
including one that edits `/members`.

```bash
cd ..
modal contract create --dir ./too-weak
cd too-weak
modal checkout
modal set-named-id /members/carol.id example/alice
modal set-named-id /members/dave.id example/bob
modal set-named-id /members/erin.id example/erin
modal add-rule --name weak \
  'always([+modifies(/members) -any_signed(/members)] false)'
```

```modality
model Contract {
  part flow {
    q0 --> q1
    q1 --> q1: +any_signed(/members)
  }
}
```

```bash
modal commit --all -m "Too weak"
modal set-named-id /members/frank.id example/frank
modal commit --all --sign example/alice -m "Carol adds Frank"
```

That commit lands. The rule verified. Carol added Frank alone. The request
was every member, and this formula does not say that.

## The idea

The checker accepts a commit that meets the rules and a witness that meets
the rules. It does not check that the rule is the request you made. Too
strong refuses a witness that still allows a commit you need. Too weak
accepts a witness that lets a forbidden commit through. The
[rule suite](../reference/ai-rule-suite) grades both shapes. Lint and a
witness you can read are how you catch them before the commit.

Next: [On the testnet](on-the-testnet).

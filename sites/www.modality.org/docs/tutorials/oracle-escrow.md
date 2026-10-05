---
sidebar_position: 13
title: Someone Else's Word
---

# Someone Else's Word

The buyer wants to release the funds only after delivery. The checker did
not see the package. Delivery has to be posted, as a commit signed by the
attestor, before a rule can use it. Until that post is in the log, the
release has no step.

`text_eq` reads the delivery note already accepted. The attestor's signature
is what makes that note their word.

```bash
modal id create --name example/erin
modal contract create --dir ./escrow
cd escrow
modal checkout
modal set-named-id /parties/buyer.id example/alice
modal set-named-id /parties/seller.id example/bob
modal set-named-id /parties/attestor.id example/erin
modal add-rule --name who \
  'always([-signed_by(/parties/buyer.id) -signed_by(/parties/seller.id) -signed_by(/parties/attestor.id)] false)'
modal add-rule --name release \
  'always([+modifies(/escrow/release.text) -text_eq(/delivery.text, "yes")] false)'
```

```modality
model Contract {
  part flow {
    q0 --> q1
    q1 --> q1: +signed_by(/parties/buyer.id) -modifies(/escrow/release.text)
    q1 --> q1: +signed_by(/parties/seller.id) -modifies(/escrow/release.text)
    q1 --> q2: +signed_by(/parties/attestor.id) +post_to_path(/delivery.text) -modifies(/escrow/release.text)
    q2 --> q2: +signed_by(/parties/buyer.id) +modifies(/escrow/release.text) +text_eq(/delivery.text, "yes")
    q2 --> q2: +signed_by(/parties/buyer.id) -modifies(/escrow/release.text)
  }
}
```

```bash
modal commit --all -m "Escrow"
modal commit \
  --path /escrow/release.text \
  --value yes \
  --sign example/alice \
  -m "Release early"
```

```output
No valid transition for local commit from current states {"q1"}
Closest candidate transition: q1 -> q1 [+signed_by(/parties/buyer.id) -modifies(/escrow/release.text)]; failed predicates: forbidden -modifies(/escrow/release.text) matched
```

The buyer is allowed to write. They are not allowed to write the release.
The attestor posts delivery, and then the buyer releases.

```bash
modal commit --path /delivery.text --value yes --sign example/erin -m "Delivered"
modal commit --path /escrow/release.text --value yes --sign example/alice -m "Release"
```

Both land. The release is a step from `q2`, and `q2` is the state after the
attestor's commit.

## The idea

A fact the checker cannot see is usable once it is accepted state. The rule
`always([+modifies(/escrow/release.text) -text_eq(/delivery.text, "yes")] false)`
forbids the release while `/delivery.text` is anything but `yes`. The
attestor's key is the one the witness allows to post that path. A predicate
such as `oracle_attests` holds only when the commit carries a replay bundle
for the claim. This page uses the posted note, which the local checker
evaluates. See [standard predicates](../reference/standard-predicates).

Next: [Their state, in yours](their-state).

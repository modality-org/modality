---
sidebar_position: 2
title: The Commit That Used to Work
---

# The Commit That Used to Work

You and a counterparty have already posted a note. You now want that note
to stay. A new rule can say so. The rule does not rewrite the commits
already in the log. It stays, and every later commit has to meet it,
including the kind of commit that just worked.

This page starts a new directory with the same shape as
[Your First Contract](../getting-started/first-contract): Alice, Bob, and a
rule that either of them may sign. If those two identities already exist,
skip creating them.

```bash
modal id create --name example/alice
modal id create --name example/bob
modal contract create --dir ./frozen-note
cd frozen-note
modal checkout
modal set-named-id /parties/alice.id example/alice
modal set-named-id /parties/bob.id example/bob
modal add-rule --name authorized \
  'always([-signed_by(/parties/alice.id) -signed_by(/parties/bob.id)] false)'
```

The witness is the cookbook model: an unlabeled first step, then Alice or
Bob.

```modality
model Contract {
  part flow {
    q0 --> q1
    q1 --> q1: +signed_by(/parties/alice.id)
    q1 --> q1: +signed_by(/parties/bob.id)
  }
}
```

Write that to `model/default.modality`, then commit the bootstrap. The
rule starts after this commit, so the bootstrap itself is unsigned.

```bash
modal commit --all -m "Initial contract setup"
modal commit --path /notes.text --value "signed update" --sign example/alice -m "Signed update"
modal commit --path /scratch.text --value "bob was here" --sign example/bob -m "Bob writes"
```

Bob's note lands. The rule allowed it. Alice's note is in the log. Both of
those commits stay there.

## Freeze the note

Add a second rule. It forbids any later commit that writes `/notes.text`.
The first rule is still in the log. Both apply.

```bash
modal add-rule --name frozen 'always([+modifies(/notes.text)] false)'
```

The witness has to replay the commits you already accepted, and its later
steps have to be unable to write the note. The commit that installs this
rule is a model change signed by Alice, and it must not itself write the
note. Later posts may still happen, as long as they leave `/notes.text`
alone.

```modality
model Contract {
  part flow {
    q0 --> q1
    q1 --> q1: +POST +signed_by(/parties/alice.id)
    q1 --> q1: +POST +signed_by(/parties/bob.id)
    q1 --> q2: +MODEL +signed_by(/parties/alice.id) -modifies(/notes.text)
    q2 --> q2: +POST +signed_by(/parties/alice.id) -modifies(/notes.text)
    q2 --> q2: +POST +signed_by(/parties/bob.id) -modifies(/notes.text)
  }
}
```

```bash
modal commit --all --sign example/alice -m "Freeze the note"
```

That commit lands. You are in `q2`. Try the commit that used to work: Alice
writing the note again.

```bash
modal commit \
  --path /notes.text \
  --value "changed" \
  --sign example/alice \
  -m "Change the note"
```

```output
No valid transition for local commit from current states {"q2"}
Closest candidate transition: q2 -> q2 [+POST +signed_by(/parties/alice.id) -modifies(/notes.text)]; failed predicates: forbidden -modifies(/notes.text) matched
```

The log did not take it. A different path still can:

```bash
modal commit --path /later.text --value "ok" --sign example/alice -m "A later note"
modal checkout
```

## The idea

A rule is checked from the state the commit that adds it reaches. Commits
already in the log stay. Every commit after the freeze has to leave
`/notes.text` alone, and a later model that grows an edge which writes that
path is refused, because the rule is still in force. That is
[accumulating rules](../reference/contract-evolution).

Next: [States, not one room](states).

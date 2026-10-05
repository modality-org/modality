---
sidebar_position: 5
title: A Finish That Stays Possible
---

# A Finish That Stays Possible

The offer from [States, not one room](states) can sit unfinished. That is
allowed. What you want the witness to show is that finishing is still
reachable. `eventually` says a path can get there. It does not make the next
commit be that path.

```bash
modal contract create --dir ./can-finish
cd can-finish
modal checkout
modal set-named-id /parties/alice.id example/alice
modal add-rule --name finishable \
  'eventually(<+post_to_path(/offer/done.text)> true)'
```

First, a witness that can never post `/offer/done.text`. Write this to
`model/default.modality` and try to commit it.

```modality
model Contract {
  part flow {
    q0 --> q1
    q1 --> q1: +signed_by(/parties/alice.id) -post_to_path(/offer/done.text)
  }
}
```

```bash
modal commit --all -m "Trapped"
```

```output
Model violates rule 'local_rule'
formula: eventually(<+post_to_path(/offer/done.text)> true)
counterexample: eventually(<+post_to_path(/offer/done.text)> true) failed because no satisfying state is reachable from q1
```

The rule did not enter the log. Replace the witness with one that can still
finish, and that can also wait.

```modality
model Contract {
  part flow {
    q0 --> q1
    q1 --> q1: +signed_by(/parties/alice.id) -post_to_path(/offer/done.text)
    q1 --> q2: +signed_by(/parties/alice.id) +post_to_path(/offer/done.text)
    q2 --> q2: +signed_by(/parties/alice.id)
  }
}
```

```bash
modal commit --all -m "Can finish"
modal commit --path /offer/note.text --value "not yet" --sign example/alice -m "Not done yet"
modal commit --path /offer/done.text --value done --sign example/alice -m "Done"
```

The middle commit lands. The rule was already in force, and the contract was
not forced to finish on that commit. The last commit is the finish the
witness still had.

## The idea

`eventually(φ)` holds when φ is true now or some path of steps reaches it.
It is reachability. A run may stop, or loop, before it gets there. No rule
can make a commit happen. `always(eventually(φ))` is the stronger request
that φ stay reachable from every later state. Both are in
[modal logic](../concepts/modal-logic). The checker refuses a witness that
has no path to the finish you required.

Next: [The rule that looks right](rule-that-looks-right).

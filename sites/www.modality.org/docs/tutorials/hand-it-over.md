---
sidebar_position: 18
title: Hand It Over
---

# Hand It Over

The last check is someone else's. They have the log. They do not have your
word for what it means, and they do not ask the node that sequenced it to
be honest. Replay runs the commits again and accepts the log when every
commit meets the model and the rules.

Use the source contract from [Their state, in yours](their-state), or any
directory whose log you did not just watch yourself write. From that
directory:

```bash
modal replay
```

```output
✓ Independent replay passed
  Commits:     3
```

The digest and the commit count are what a second directory should print
too. Copy the contract directory, or pull it on another machine, and run
`modal replay` there. The same digest is the same log.

A file saved from a sequencer is the same check, with no network:

```bash
modal replay --artifact prefix.json
```

`--through` stops at a sequenced commit. The artifact names the predicate
theory it was checked under. A local directory is checked under v3 unless
you pass `--theory`.

If replay fails, the log is not the contract you think it is. A commit the
witness refuses, a rule added and then ignored, or a body that does not
hash to its id, stops the check. The series ends when the second directory
prints that it passed.

## The idea

Independent replay is the accept rule run by someone other than the
author. "Verified" means the commits conform to the rules in the log.
Whether those rules match what a person hoped is the gap in
[Too strong, then too weak](too-strong-too-weak). The command is
[`modal replay`](../cli/contract-commands.md).

You can go back through the [series](./index), or into the
[language](../language) and the [network](../cli/join-testnet).

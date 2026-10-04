---
sidebar_position: 14
title: Their State, in Yours
---

# Their State, in Yours

Another contract has posted a price. Yours should carry that price as they
committed it. A repost copies one path from a
source commit into your log. Your rules still have to accept the commit
that contains it.

Build a source and post the price. Alice signs, as in the earlier pages.

```bash
modal contract create --dir ./price-source
cd price-source
modal checkout
modal set-named-id /parties/alice.id example/alice
modal add-rule --name authorized \
  'always([-signed_by(/parties/alice.id)] false)'
```

```modality
model Contract {
  part flow {
    q0 --> q1
    q1 --> q1: +signed_by(/parties/alice.id)
  }
}
```

```bash
modal commit --all -m "Source"
modal commit --path /facts/price.text --value "10 units" --sign example/alice -m "Price"
modal status
```

Copy the source contract id. It is the Modality spelling.

The destination is Bob's contract. A repost is a commit, and this witness
allows Bob's signature on any later step, including the import.

```bash
cd ..
modal contract create --dir ./price-dest
cd price-dest
modal checkout
modal set-named-id /parties/bob.id example/bob
modal add-rule --name authorized 'always([-signed_by(/parties/bob.id)] false)'
```

```modality
model Contract {
  part flow {
    q0 --> q1
    q1 --> q1: +signed_by(/parties/bob.id)
  }
}
```

Commit the bootstrap.

```bash
modal commit --all -m "Dest"
modal repost <source contract id> /facts/missing.text --from-dir ../price-source
```

```output
Path '/facts/missing.text' not found in source contract <source contract id>
```

Nothing was staged. The source has `/facts/price.text`. Repost that path.
`--from-dir` reads a local log. Without it, `repost` reads the hub saved as
`origin`. A source commit that has not been sequenced is refused when the
destination is checking a remote source: the snapshot is of a commit the
network has ordered.

```bash
modal repost <source contract id> /facts/price.text --from-dir ../price-source
modal commit --all --sign example/bob -m "Take their price"
modal checkout
cat state/reposts/<source contract id>/facts/price.text
```

The file is `10 units`. The commit records the source contract, the source
path, and the source commit id beside the value. After it is accepted, the
destination path is ordinary state.

## The idea

A repost is a copy of one path at one source commit. The value is theirs,
from that commit. The method label is `+REPOST`. A
witness that only allows `+POST` has to allow `+REPOST` too, or leave the
step unlabeled, before the import commits. The destination's rules still
run. See [REPOST](../reference/commit-methods.md#repost). On the testnet,
the source commit is one that has sequenced. [Contract Hub](contract-hub)
is the same copy between two people.

Next: [Commit, then show](commit-then-show).

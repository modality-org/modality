# Push admission: size and rate limits on `/contract/push`

A sequencer with tight `push_limits` in its `config.json`:

- A push that carries more than `max_commits` commits is refused.
- A contract that has pushed `commits_per_contract_per_minute` commits is
  refused until its minute refills, whichever peer sends them.
- Another contract still pushes.

A local filter, not consensus: each node chooses what it stores and queues.
A refused push is not charged and can be retried.

```bash
./test.sh
```

Unnumbered: not part of the stable numbered suite.

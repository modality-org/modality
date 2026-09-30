# Agent Swarm: Coordinator and Workers

A coordinator posts a task, any worker on the team claims it and submits a
result, and the coordinator approves and pays. The team is fixed at setup:
`/coordinator.id` and the `.id` files under `/workers`.

## Contract Rules

```modality
// team_fixed
always(([+modifies(/workers)] false) & ([+modifies(/coordinator.id)] false))
// coordinator_posts_approves_pays
always(([+modifies(/task/posted.bool) -signed_by(/coordinator.id)] false) & ([+modifies(/task/approved.bool) -signed_by(/coordinator.id)] false) & ([+modifies(/task/paid.bool) -signed_by(/coordinator.id)] false))
// a_worker_claims_and_submits
always(([+modifies(/task/claimed.bool) -any_signed(/workers)] false) & ([+modifies(/task/submitted.bool) -any_signed(/workers)] false))
// claimed_once
always([+modifies(/task/claimed.bool) +bool_true(/task/claimed.bool)] false)
// in_order
always(([+modifies(/task/claimed.bool) -bool_true(/task/posted.bool)] false) & ([+modifies(/task/submitted.bool) -bool_true(/task/claimed.bool)] false) & ([+modifies(/task/approved.bool) -bool_true(/task/submitted.bool)] false) & ([+modifies(/task/paid.bool) -bool_true(/task/approved.bool)] false))
// paid_once
always([+modifies(/task/paid.bool) +bool_true(/task/paid.bool)] false)
```

`any_signed(/workers)` holds when any key under `/workers` signed the commit,
so the rules name the team, not a particular worker.

## Walkthrough

The commands run in order from an empty directory.

### 1. Start a hub; the coordinator sets up the swarm

```bash
mkdir swarm-demo && cd swarm-demo
modal hub start --host 127.0.0.1 --port 8080 --rpc-port 0 --data-dir .hub &
HUB=http://127.0.0.1:8080
sleep 2
for who in coord w1 w2 outsider; do modal id create --path $who.passfile; done

modal c create --dir coord
cd coord
modal c set-named-id /coordinator.id ../coord.passfile
modal c set-named-id /workers/w1.id ../w1.passfile
modal c set-named-id /workers/w2.id ../w2.passfile
cat > model/default.modality <<'EOF'
model swarm_task {
  part flow {
    q0 --> q1
    q1 --> q1: +signed_by(/coordinator.id) -bool_true(/task/posted.bool) +modifies(/task/posted.bool) -modifies(/task/claimed.bool) -modifies(/task/submitted.bool) -modifies(/task/approved.bool) -modifies(/task/paid.bool) -modifies(/workers) -modifies(/coordinator.id)
    q1 --> q1: +any_signed(/workers) +bool_true(/task/posted.bool) -bool_true(/task/claimed.bool) +modifies(/task/claimed.bool) -modifies(/task/posted.bool) -modifies(/task/submitted.bool) -modifies(/task/approved.bool) -modifies(/task/paid.bool) -modifies(/workers) -modifies(/coordinator.id)
    q1 --> q1: +any_signed(/workers) +bool_true(/task/claimed.bool) +modifies(/task/submitted.bool) -modifies(/task/posted.bool) -modifies(/task/claimed.bool) -modifies(/task/approved.bool) -modifies(/task/paid.bool) -modifies(/workers) -modifies(/coordinator.id)
    q1 --> q1: +signed_by(/coordinator.id) +bool_true(/task/submitted.bool) +modifies(/task/approved.bool) -modifies(/task/posted.bool) -modifies(/task/claimed.bool) -modifies(/task/submitted.bool) -modifies(/task/paid.bool) -modifies(/workers) -modifies(/coordinator.id)
    q1 --> q1: +signed_by(/coordinator.id) +bool_true(/task/approved.bool) -bool_true(/task/paid.bool) +modifies(/task/paid.bool) -modifies(/task/posted.bool) -modifies(/task/claimed.bool) -modifies(/task/submitted.bool) -modifies(/task/approved.bool) -modifies(/workers) -modifies(/coordinator.id)
  }
}
EOF
modal add-rule --name team_fixed 'always(([+modifies(/workers)] false) & ([+modifies(/coordinator.id)] false))'
modal add-rule --name coordinator_posts_approves_pays 'always(([+modifies(/task/posted.bool) -signed_by(/coordinator.id)] false) & ([+modifies(/task/approved.bool) -signed_by(/coordinator.id)] false) & ([+modifies(/task/paid.bool) -signed_by(/coordinator.id)] false))'
modal add-rule --name a_worker_claims_and_submits 'always(([+modifies(/task/claimed.bool) -any_signed(/workers)] false) & ([+modifies(/task/submitted.bool) -any_signed(/workers)] false))'
modal add-rule --name claimed_once 'always([+modifies(/task/claimed.bool) +bool_true(/task/claimed.bool)] false)'
modal add-rule --name in_order 'always(([+modifies(/task/claimed.bool) -bool_true(/task/posted.bool)] false) & ([+modifies(/task/submitted.bool) -bool_true(/task/claimed.bool)] false) & ([+modifies(/task/approved.bool) -bool_true(/task/submitted.bool)] false) & ([+modifies(/task/paid.bool) -bool_true(/task/approved.bool)] false))'
modal add-rule --name paid_once 'always([+modifies(/task/paid.bool) +bool_true(/task/paid.bool)] false)'
modal c commit --all --sign ../coord.passfile -m "Swarm setup"
modal c commit --path /task/posted.bool --value true --sign ../coord.passfile -m "Post task: summarize the dataset"
CONTRACT=$(modal c id)
modal c push --remote $HUB/contracts/$CONTRACT
cd ..
```

### 2. An outsider cannot claim; a worker can

```bash
modal c pull $HUB/contracts/$CONTRACT --dir w1
cd w1
if modal c commit --path /task/claimed.bool --value true --sign ../outsider.passfile -m "Claim"; then
  echo "unexpected: an outsider claimed" && exit 1
fi
echo "refused: only a worker claims"
modal c commit --path /task/claimed.bool --value true --sign ../w1.passfile -m "Claim"
modal c commit --path /task/submitted.bool --value true --sign ../w1.passfile -m "Submit result"
modal c push
cd ..
```

### 3. The coordinator approves and pays, once

```bash
cd coord
modal c pull
modal c commit --path /task/approved.bool --value true --sign ../coord.passfile -m "Approve"
modal c commit --path /task/paid.bool --value true --sign ../coord.passfile -m "Pay"
if modal c commit --path /task/paid.bool --value true --sign ../coord.passfile -m "Pay again"; then
  echo "unexpected: paid twice" && exit 1
fi
echo "refused: the task is paid once"
modal c push
cd ..
kill %1
```

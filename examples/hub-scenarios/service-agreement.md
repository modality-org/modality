# Service Agreement: Client and Provider

A client and a provider run a milestone through a hub. Both activate the
agreement, the provider submits the milestone, the client approves it, and
both sign completion. Either may terminate before completion. Each step is a
signed commit that posts a flag; the rules say who may post each flag and what
must come first.

## Contract Rules

```modality
// parties_fixed
always([+modifies(/parties)] false)
// both_activate
always(([+modifies(/agreement/active.bool) -signed_by(/parties/client.id)] false) & ([+modifies(/agreement/active.bool) -signed_by(/parties/provider.id)] false))
// provider_submits_when_active
always(([+modifies(/milestone/submitted.bool) -signed_by(/parties/provider.id)] false) & ([+modifies(/milestone/submitted.bool) -bool_true(/agreement/active.bool)] false))
// client_approves_submitted
always(([+modifies(/milestone/approved.bool) -signed_by(/parties/client.id)] false) & ([+modifies(/milestone/approved.bool) -bool_true(/milestone/submitted.bool)] false))
// both_complete_after_approval
always(([+modifies(/agreement/completed.bool) -signed_by(/parties/client.id)] false) & ([+modifies(/agreement/completed.bool) -signed_by(/parties/provider.id)] false) & ([+modifies(/agreement/completed.bool) -bool_true(/milestone/approved.bool)] false))
// either_terminates_before_completion
always(([+modifies(/agreement/terminated.bool) -signed_by(/parties/client.id) -signed_by(/parties/provider.id)] false) & ([+modifies(/agreement/terminated.bool) +bool_true(/agreement/completed.bool)] false))
// nothing_after_termination
always([+bool_true(/agreement/terminated.bool)] false)
```

For several milestones, post `/milestones/<n>/submitted.bool` and
`/milestones/<n>/approved.bool` and write the rules with a path variable
(`/milestones/$n/...`); see the formula cookbook.

## Walkthrough

The commands run in order from an empty directory.

### 1. Start a hub; the client drafts the agreement

```bash
mkdir service-demo && cd service-demo
modal hub start --host 127.0.0.1 --port 8080 --rpc-port 0 --data-dir .hub &
HUB=http://127.0.0.1:8080
sleep 2
for who in client provider; do modal id create --path $who.passfile; done

modal c create --dir client
cd client
modal c set-named-id /parties/client.id ../client.passfile
modal c set-named-id /parties/provider.id ../provider.passfile
cat > model/default.modality <<'EOF'
model service_agreement {
  part flow {
    q0 --> q1
    q1 --> q1: +signed_by(/parties/client.id) +signed_by(/parties/provider.id) -bool_true(/agreement/terminated.bool) +modifies(/agreement/active.bool) -modifies(/milestone/submitted.bool) -modifies(/milestone/approved.bool) -modifies(/agreement/completed.bool) -modifies(/agreement/terminated.bool) -modifies(/parties)
    q1 --> q1: +signed_by(/parties/provider.id) +bool_true(/agreement/active.bool) -bool_true(/agreement/terminated.bool) +modifies(/milestone/submitted.bool) -modifies(/agreement/active.bool) -modifies(/milestone/approved.bool) -modifies(/agreement/completed.bool) -modifies(/agreement/terminated.bool) -modifies(/parties)
    q1 --> q1: +signed_by(/parties/client.id) +bool_true(/milestone/submitted.bool) -bool_true(/agreement/terminated.bool) +modifies(/milestone/approved.bool) -modifies(/agreement/active.bool) -modifies(/milestone/submitted.bool) -modifies(/agreement/completed.bool) -modifies(/agreement/terminated.bool) -modifies(/parties)
    q1 --> q1: +signed_by(/parties/client.id) +signed_by(/parties/provider.id) +bool_true(/milestone/approved.bool) -bool_true(/agreement/terminated.bool) +modifies(/agreement/completed.bool) -modifies(/agreement/active.bool) -modifies(/milestone/submitted.bool) -modifies(/milestone/approved.bool) -modifies(/agreement/terminated.bool) -modifies(/parties)
    q1 --> q1: +signed_by(/parties/client.id) -bool_true(/agreement/completed.bool) -bool_true(/agreement/terminated.bool) +modifies(/agreement/terminated.bool) -modifies(/agreement/active.bool) -modifies(/milestone/submitted.bool) -modifies(/milestone/approved.bool) -modifies(/agreement/completed.bool) -modifies(/parties)
    q1 --> q1: +signed_by(/parties/provider.id) -bool_true(/agreement/completed.bool) -bool_true(/agreement/terminated.bool) +modifies(/agreement/terminated.bool) -modifies(/agreement/active.bool) -modifies(/milestone/submitted.bool) -modifies(/milestone/approved.bool) -modifies(/agreement/completed.bool) -modifies(/parties)
  }
}
EOF
modal add-rule --name parties_fixed 'always([+modifies(/parties)] false)'
modal add-rule --name both_activate 'always(([+modifies(/agreement/active.bool) -signed_by(/parties/client.id)] false) & ([+modifies(/agreement/active.bool) -signed_by(/parties/provider.id)] false))'
modal add-rule --name provider_submits_when_active 'always(([+modifies(/milestone/submitted.bool) -signed_by(/parties/provider.id)] false) & ([+modifies(/milestone/submitted.bool) -bool_true(/agreement/active.bool)] false))'
modal add-rule --name client_approves_submitted 'always(([+modifies(/milestone/approved.bool) -signed_by(/parties/client.id)] false) & ([+modifies(/milestone/approved.bool) -bool_true(/milestone/submitted.bool)] false))'
modal add-rule --name both_complete_after_approval 'always(([+modifies(/agreement/completed.bool) -signed_by(/parties/client.id)] false) & ([+modifies(/agreement/completed.bool) -signed_by(/parties/provider.id)] false) & ([+modifies(/agreement/completed.bool) -bool_true(/milestone/approved.bool)] false))'
modal add-rule --name either_terminates_before_completion 'always(([+modifies(/agreement/terminated.bool) -signed_by(/parties/client.id) -signed_by(/parties/provider.id)] false) & ([+modifies(/agreement/terminated.bool) +bool_true(/agreement/completed.bool)] false))'
modal add-rule --name nothing_after_termination 'always([+bool_true(/agreement/terminated.bool)] false)'
modal c commit --all --sign ../client.passfile -m "Service agreement draft"
CONTRACT=$(modal c id)
modal c push --remote $HUB/contracts/$CONTRACT
cd ..
```

### 2. The provider takes a copy; both sign activation

A commit may carry several signatures. Here the provider signs a commit that
the client also signs, on one copy.

```bash
modal c pull $HUB/contracts/$CONTRACT --dir provider
cd provider
if modal c commit --path /milestone/submitted.bool --value true --sign ../provider.passfile -m "Milestone 1"; then
  echo "unexpected: submitted before activation" && exit 1
fi
echo "refused: the agreement is not active yet"
modal c commit --path /agreement/active.bool --value true --sign ../client.passfile --sign ../provider.passfile -m "Activate"
modal c push
```

### 3. The provider submits; the client approves

```bash
modal c commit --path /milestone/submitted.bool --value true --sign ../provider.passfile -m "Milestone 1"
modal c push
cd ../client
modal c pull
modal c commit --path /milestone/approved.bool --value true --sign ../client.passfile -m "Approve milestone 1"
modal c push
cd ..
```

### 4. Both sign completion; termination is then refused

```bash
cd provider
modal c pull
modal c commit --path /agreement/completed.bool --value true --sign ../client.passfile --sign ../provider.passfile -m "Complete"
modal c push
if modal c commit --path /agreement/terminated.bool --value true --sign ../provider.passfile -m "Terminate"; then
  echo "unexpected: terminated after completion" && exit 1
fi
echo "refused: a completed agreement is not terminated"
cd ..
kill %1
```

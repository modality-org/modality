# Members-Only Contract

Only members can commit. Changing the membership takes every current member's
signature. The rules are part of the contract: every copy checks them on every
commit, and they cannot be removed.

## State Structure

```
/members/
  alice.id → alice's Modality ID
  bob.id → bob's Modality ID
  carol.id → carol's Modality ID
```

## Contract Rules

```modality
// Every commit after the rules are added is signed by a member
always([-any_signed(/members)] false)

// A commit that changes /members is signed by every current member
always([+modifies(/members) -all_signed(/members)] false)
```

`all_signed(/members)` reads the members in accepted state, so the second rule
asks for more signatures as the membership grows. The rules name what a commit
does (`modifies(/members)`), not an action label, so no model can dodge them
by calling the change something else.

## Walkthrough

The commands run in order from an empty directory.

### 1. Start a hub; Alice creates the contract

```bash
mkdir members-demo && cd members-demo
modal hub start --host 127.0.0.1 --port 8080 --rpc-port 0 --data-dir .hub &
HUB=http://127.0.0.1:8080
sleep 2
for who in alice bob carol eve; do modal id create --path $who.passfile; done

modal c create --dir alice
cd alice
modal c set-named-id /members/alice.id ../alice.passfile
cat > model/default.modality <<'EOF'
model members_only {
  part flow {
    q0 --> q1
    q1 --> q1: +any_signed(/members) -modifies(/members)
    q1 --> q1: +any_signed(/members) +all_signed(/members)
  }
}
EOF
modal add-rule --name member_required 'always([-any_signed(/members)] false)'
modal add-rule --name membership_unanimous 'always([+modifies(/members) -all_signed(/members)] false)'
modal c commit --all --sign ../alice.passfile -m "Members-only contract"
CONTRACT=$(modal c id)
modal c push --remote $HUB/contracts/$CONTRACT
```

### 2. Alice adds Bob; her signature is every member's

```bash
modal c set-named-id /members/bob.id ../bob.passfile
modal c commit --all --sign ../alice.passfile -m "Add Bob"
modal c push
cd ..
```

### 3. Bob takes a copy; adding Carol takes both of them

```bash
modal c pull $HUB/contracts/$CONTRACT --dir bob
cd bob
modal c set-named-id /members/carol.id ../carol.passfile
if modal c commit --all --sign ../bob.passfile -m "Add Carol"; then
  echo "unexpected: one member added another" && exit 1
fi
echo "refused: Alice must sign too"
modal c commit --all --sign ../bob.passfile --sign ../alice.passfile -m "Add Carol"
modal c push
```

### 4. A member posts; a stranger cannot

```bash
modal c commit --path /notes/agenda.md --value "# Agenda" --sign ../bob.passfile -m "Agenda"
modal c push
if modal c commit --path /notes/spam.md --value "spam" --sign ../eve.passfile -m "Spam"; then
  echo "unexpected: a non-member committed" && exit 1
fi
echo "refused: Eve is not a member"
cd ..
kill %1
```

## Key Points

1. **Rules accumulate.** Each rule commit must leave a model that meets every
   rule, old and new, so a later model cannot drop the membership rule.
2. **The rule's meaning follows state.** `all_signed(/members)` counts the
   members at the time of the commit.
3. **Predicates, not labels.** The rules speak of paths and signatures.

# First Contract Fixture

This parser-backed fixture is the source of truth for the canonical first
contract path while the onboarding docs are brought into line with the working
CLI.

Run it from this directory:

```bash
./test-validate.sh
```

The check verifies that `modality model validate` accepts the witness model with
one part, four transitions, and predicate-only transition properties.

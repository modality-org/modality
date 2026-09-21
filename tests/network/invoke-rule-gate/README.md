# Invoke still gated by accumulated rules

Unsigned WASM that emits `POST` is rejected on sequenced apply. Alice-signed
invoke of the same program is accepted. A stranger can replay the prefix,
including the posted WASM.

```bash
./test.sh
```

Unnumbered: not part of the stable numbered suite.

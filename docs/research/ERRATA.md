# Errata

Supersessions of `docs/research/`. One line each. Never edit the originals.

Format: `<document> §<section> -> superseded by <RFC> §<section> (<reason>)`

---

- `00-jit-feasibility.md` §4.1 -> superseded by the OOP finding in
  `03-metaprogramming-oop-debugger.md` §2.2: `.id` and `.timestamp` are public
  API on the base `Object` class, so the value cannot shrink to 16 bytes.

- `02-native-binaries.md` §4 -> superseded by `03-metaprogramming-oop-debugger.md`
  §0.2: `bund.eval` exists, so closed-world analysis rarely fires and the front
  end is mandatory in every binary.

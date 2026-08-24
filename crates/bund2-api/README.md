# bund2-api

**This crate is the only stability guarantee in the workspace.**

External Rust word packages depend on this crate and nothing else from Bund2.
Every change to its public surface is a breaking change and needs a version
bump and a migration note, regardless of how small it looks.

Everything else in the workspace may churn freely. If you are tempted to widen
this crate so that something internal becomes reachable, widen the internal
crate instead and leave this one alone.

Specifically out of scope: Cranelift types. Exposing `FunctionBuilder` here
would pin every third-party package to an exact Cranelift version, which turns
the guarantee above into a false one. See RFC-0002.

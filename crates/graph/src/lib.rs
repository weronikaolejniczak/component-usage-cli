//! Builds the module graph from file summaries: resolves import specifiers and follows re-exports
//! to the module that defines each export.
//!
//! Reads `package.json` and `tsconfig.json` through the resolver; no other I/O.

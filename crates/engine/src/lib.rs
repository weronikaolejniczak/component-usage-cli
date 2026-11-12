//! Runs a scan: discovers files, parses them in parallel, builds the module graph, identifies usages.
//!
//! The only library crate that walks the filesystem and the only one that uses threads.

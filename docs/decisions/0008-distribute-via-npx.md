---
status: accepted
date: 2026-11-12
decision-makers: Weronika Olejniczak
---

# Distribute through npm with per-platform binaries

## Context and Problem Statement

Users are JS developers. How do they install and run a Rust binary?

## Decision Drivers

* Runs with `npx component-usage-cli` and no extra setup.
* No Rust toolchain on the user's machine.

## Considered Options

* Per-platform npm packages with prebuilt binaries and a JS launcher
* napi-rs Node addon
* `cargo install` only
* GitHub release binaries only

## Decision Outcome

Chosen option: "Per-platform npm packages", the pattern used by esbuild, Biome and oxlint.

* One package per platform holding the binary, with `os` and `cpu` set.
* Main package `component-usage-cli` lists them in `optionalDependencies` and contains a launcher that runs the installed binary.
* GitHub Actions builds all targets and publishes all packages at the same version on a tag.

### Consequences

* Good, because `npx component-usage-cli` works without Rust.
* Bad, because every release publishes several packages.

## Pros and Cons of the Options

### napi-rs Node addon

* Good, because the scanner could be called from JS.
* Bad, because there's no JS API to expose; it's a CLI.

### `cargo install` / GitHub release binaries only

* Bad, because most users don't have Rust or won't download binaries by hand.
* Neutral, because both can be offered in addition.

## More Information

npm name `component-usage-cli`. `component-usage` belongs to another npm account (published and unpublished in 2024), and `component-scanner` is taken. Repository: [weronikaolejniczak/component-usage-cli](https://github.com/weronikaolejniczak/component-usage-cli), personal, not under an organization.

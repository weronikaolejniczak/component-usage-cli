---
status: accepted
date: 2026-11-19
decision-makers: Weronika Olejniczak
---

# Report operators with their operands

Supersedes, in [ADR-0017](./0017-prop-values.md), that an expression with an operator, such as `a + b`, is `unresolved` with the code `unsupported`.

## Context and Problem Statement

Props such as `isDisabled={!canSave}` or `isInvalid={errors.length > 0}` were `unsupported`, so the report said nothing about them: not that they're the result of an operator, nor what the operands are. They are the largest group of `unsupported` values on the codebases in [the benchmarks](../../benchmarks/README.md). How should an expression with an operator be reported?

## Considered Options

* An `operation` with the operator and its operands
* Evaluate operators whose operands are literals
* The type of the result, without the operands
* As before

## Decision Outcome

Chosen option: "An `operation` with the operator and its operands", because it keeps what is written, and each operand is followed and positioned like any other value.

* A unary or binary operator is an `operation`, with `operator`, as written, and `operands`, the values of its operands in source order.
* Unary operators are `!`, `-`, `+`, `~`, `typeof` and `delete`; binary ones are the arithmetic, bitwise, comparison and equality operators, `in` and `instanceof`.
* Operations aren't evaluated, even when every operand is a literal.
* `void` stays `undefined`, and `-` before a number literal stays a `literal` ([ADR-0017](./0017-prop-values.md)). `?:`, `||`, `&&` and `??` stay `one-of`.
* Other expressions, such as `new X()` or `#field in object`, stay `unsupported`.
* Spreading an operation may set any prop in the summary ([ADR-0033](./0033-summary-spreads.md)); its operands aren't counted as values of the prop.

### Consequences

* Good, because a prop computed with an operator says so, and its operands can be followed to variables, props and calls.
* Good, because the report doesn't state a type it can't be sure of: `-` and `*` may give a `bigint`, and `+` a string.
* Bad, because the result of an operator isn't known even when it could be computed, as in `!true`.

### Confirmation

A props fixture with each unary operator, binary operators of each group, operands that are followed, nested operations, and expressions that stay `unsupported`.

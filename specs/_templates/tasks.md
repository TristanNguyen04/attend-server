# Tasks: <feature name>

Derived from `requirements.md` and `tests/features/<module>/<feature>.feature`.
One task per scenario, in the constitution's order:
**BDD red -> TDD red -> TDD green -> BDD green**.

---

## T1: <scenario title>  ·  FR<n>
Scenario: *<scenario title>*

- **BDD red**: write the step definitions for the whole scenario against real behaviour.
  Run `cargo test --test cucumber`; it must fail because <the expected reason>.
- **TDD red**: write a failing unit test that <asserts the smallest missing behaviour>.
- **TDD green**: <the minimal implementation>.
- **BDD green**: run `cargo test --test cucumber`; the scenario passes.

## T2: ...

---

## Cross-cutting notes
- <ordering constraints, prerequisites such as a migration, shared helpers>

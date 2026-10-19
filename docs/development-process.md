# Development process: SDD, BDD, TDD

We build every feature in three layers, each driving the next.

| Layer | Question it answers | Artefact | Lives in |
|---|---|---|---|
| **SDD** (spec-driven) | What must the system do, and why? | Requirements, decisions, tasks | `specs/<module>/<feature>/` |
| **BDD** (behaviour-driven) | How does a user see it behave? | Gherkin scenarios and step definitions | `tests/features/<module>/`, `tests/steps/` |
| **TDD** (test-driven) | Does this unit of code do its job? | Unit tests next to the code | `#[cfg(test)]` inside `src/` |

## The loop for one feature

1. **Spec.** Write `specs/<module>/<feature>/requirements.md` from the template: the user
   story, definitions, numbered requirements (FR1, FR2, ... in EARS form), non-functional
   requirements and a table of decisions. A human reviews and commits it. Specs change only
   through a reviewed commit.
2. **Scenarios.** Write `tests/features/<module>/<feature>.feature`. Cover the happy path,
   the sad paths and the permission cases. Tag each scenario with its story and requirement,
   for example `@US-SES-01 @FR2`.
3. **Tasks.** Write `specs/<module>/<feature>/tasks.md`: one task per scenario, each listing
   its four steps. Order tasks so shared groundwork (a migration, a table) lands when first
   needed.
4. **Implement one task at a time**, in this order (the constitution):

   | Step | What you do | Command | Expected |
   |---|---|---|---|
   | BDD red | Write the scenario's step definitions against real behaviour | `cargo test --test cucumber` | Fails, for the right reason |
   | TDD red | Write a failing unit test for the smallest missing piece | `cargo test --lib` | Fails |
   | TDD green | Write the minimum code to pass it | `cargo test --lib` | Passes |
   | (refactor) | Clean up while the tests stay green | `cargo test --lib` | Passes |
   | BDD green | Run the scenario again | `cargo test --test cucumber` | Passes |

   Repeat the TDD steps as often as the scenario needs. Never move to the next task while a
   test is failing.
5. **Done** when all of these hold: `cargo fmt --all -- --check`, `cargo clippy
   --all-targets -- -D warnings`, `cargo test` (this runs unit tests, the architecture test
   and Cucumber) are green, and the pull request is reviewed.

## Traceability

Every line of behaviour can be followed back:

`user story (US-...) -> requirement (FR...) -> scenario (@tags) -> task (T...) -> tests`

If you cannot say which requirement a piece of code serves, either the spec is missing a
requirement or the code is not needed.

## Commands

```
docker compose up -d                  # local Postgres (dev + test databases)
cargo test                            # everything
cargo test --lib                      # unit tests only (fast TDD loop)
cargo test --test cucumber            # BDD scenarios
cargo test --test architecture        # module dependency rules
cargo fmt --all                       # format
cargo clippy --all-targets -- -D warnings
cargo loco start                      # run the app locally
```

Two more commands mirror what CI does (CI runs them in place of plain `cargo test`):

```
# Cucumber report: also writes cucumber.json and cucumber.xml to the directory
CUCUMBER_REPORT_DIR=target/cucumber-report cargo test --test cucumber

# Coverage: runs all tests, then fails below 100% of lines of the hand-written code
rustup component add llvm-tools-preview && cargo install cargo-llvm-cov --locked   # once
RUSTFLAGS="-C link-dead-code" cargo llvm-cov --no-report
cargo llvm-cov report --ignore-filename-regex 'src/(modules/[^/]+/entities/|bin/main\.rs$)' \
  --fail-under-lines 100
cargo llvm-cov report --html              # optional: browse uncovered lines in target/llvm-cov/html
```

The regex leaves out only code we do not write by hand: the generated SeaORM entities and the
thin binary entry point. Keep it in sync with `COVERAGE_IGNORE_REGEX` in
`.github/workflows/ci.yml`. `-C link-dead-code` makes a function that no test calls show up as
uncovered instead of disappearing from the report. The `migration` crate is not measured (the
report covers the `attend_server` package only; the schema tests exercise the migrations). Write a test for new code rather than
widening the regex.

## Where tests go

| Kind | Location |
|---|---|
| Unit tests (TDD) | In the same file as the code, in `#[cfg(test)] mod tests` |
| Scenarios (BDD) | `tests/features/<module>/<feature>.feature` |
| Step definitions | `tests/steps/<module>.rs`; shared steps in `tests/steps/common.rs` |
| Architecture rules | `tests/architecture.rs` |
| Database schema (constraints, keys, value sets) | `tests/schema.rs` (no scenario: persistence has no user-visible behaviour) |

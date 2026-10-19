# Development process: spec, scenarios, tasks, tests

We keep the course's artefacts and drop its strict loop. Each artefact answers one question:

| Artefact | Question it answers | Lives in |
|---|---|---|
| **Spec** | What must the system do, and why? | `specs/<module>/<feature>/requirements.md` |
| **Scenarios** (Gherkin) | How does a user see it behave? | `tests/features/<module>/`, steps in `tests/steps/` |
| **Tasks** | What are the work items, in what order? | `specs/<module>/<feature>/tasks.md` |
| **Unit tests** | Does this piece of code do its job? | `#[cfg(test)]` inside `src/` |

## The flow for one feature

1. **Spec.** Write `specs/<module>/<feature>/requirements.md` from the template: the user
   story, definitions, numbered requirements (FR1, FR2, ... in EARS form), non-functional
   requirements and a table of decisions. A human reviews it. Specs change only through a
   reviewed commit.
2. **Scenarios.** Write `tests/features/<module>/<feature>.feature`: the happy path, the sad
   paths and the permission cases. Tag each scenario with its requirement, for example
   `@FR2`.
3. **Tasks.** Write `specs/<module>/<feature>/tasks.md`: a short list of work items, each
   naming the requirements and scenarios it covers. Group them so that shared groundwork (a
   migration, configuration) lands in the first item that needs it.
4. **Build.** Implement a task or the whole feature in one go: code, step definitions and unit
   tests together, in whatever order is fastest. Run the quick checks as you go.
5. **Done** when `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings` and
   `cargo test` (unit tests, the architecture test and all Cucumber scenarios) are green, line
   coverage of hand-written code is 100% in CI, and the pull request is reviewed.

## Why we dropped the strict loop

We tried the course's method literally: for every scenario, write the steps and watch them
fail (BDD red), write a failing unit test (TDD red), make it pass (TDD green), re-run the
scenario (BDD green), one task per prompt, with evidence of every red. **It was way too slow for
project development.** It multiplied builds and test runs, forced a review round trip per
scenario, and added nothing the CI gates do not already guarantee: a scenario that does not
assert the right thing, or code that no test covers, is caught by the scenarios themselves and
by the 100% coverage gate.

What stays: the spec with numbered requirements, the `.feature` files with tags, the task list,
tests for every behaviour, and the CI gates. What goes: the mandatory red/green order, the
one-scenario-per-prompt rule, and the per-step evidence.

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

## Where tests go

| Kind | Location |
|---|---|
| Unit tests (TDD) | In the same file as the code, in `#[cfg(test)] mod tests` |
| Scenarios (BDD) | `tests/features/<module>/<feature>.feature` |
| Step definitions | `tests/steps/<module>.rs`; shared steps in `tests/steps/common.rs` |
| Architecture rules | `tests/architecture.rs` |
| Database schema (constraints, keys, value sets) | `tests/schema.rs` (no scenario: persistence has no user-visible behaviour) |

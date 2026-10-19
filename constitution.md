# Constitution

The rules every contribution follows, human or AI. We use the course's artefacts (specs,
Gherkin scenarios, tasks, tests) but not its strict step-by-step loop: see "Why we dropped the
strict loop" in `docs/development-process.md`.

- Every feature has a spec (`requirements.md`), Gherkin scenarios (`.feature` files) and a task
  list. A human reviews the requirements and the decisions before the work starts.
- Every behaviour ships with tests in the same pull request: scenarios for what a user sees,
  unit tests for the logic. The order of writing code and tests is the author's choice.
- The gates are in CI, not in ceremony: format, lint, all tests, the Cucumber report and 100%
  line coverage of hand-written code. A pull request does not merge while one is red.
- Build in slices: a whole feature, or a coherent part of it, per task or prompt, not one
  scenario at a time.
- No breaking changes to existing endpoints.
- Spec changes only through a human-reviewed commit.
- Answer in the repo's existing stack and conventions (Loco, SeaORM, Axum).

# Feature files

Gherkin scenarios that describe behaviour in the language of the domain. They are executable
by `cargo test --test cucumber`.

## Layout

```
tests/features/
├── smoke.feature              # platform check, no module
└── <module>/<feature>.feature # one file per user story
```

Step definitions live in `tests/steps/<module>.rs`. Steps used by many features go in
`tests/steps/common.rs`.

## Conventions

- **One feature per user story.** Start with the story: `As a ... / I want ... / So that ...`.
- **Tag** the feature with the module and story, and each scenario with its requirement:
  `@capture @US-SES-06` on the feature, `@FR2` on a scenario.
- **Cover three kinds of scenario**: the happy path, the sad paths (invalid input, conflicts,
  closed windows), and the permission cases (who may *not* do this).
- **One `When` per scenario.** It is the single action under test.
- **Use domain language and observable results**, not implementation details. Say "the
  session state is CLOSED", not "the row has status 3". Do not mention URLs or JSON fields in
  the scenario text; the steps handle that.
- **Use a `Scenario Outline`** for the same behaviour with several equivalent values, not
  copies of a scenario.
- **Name actors consistently** (for example Professor Lim, Alex the student, Maya the TA).
- **Keep scenarios independent.** Each one sets up what it needs in its `Given` steps; none
  relies on another scenario having run.
- **Make time explicit.** Use the injectable clock (`shared_lib::clock`); never sleep.

## Order of work

Write the scenarios, then build the feature with its steps and unit tests together. There is no
mandatory fail-first order. See `docs/development-process.md`.

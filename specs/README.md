# Specs

One folder per feature, grouped by the module that owns it. Work that spans modules (for example
persistence or deployment) lives under `specs/platform/`:

```
specs/<module>/<feature>/
├── requirements.md   # what and why (reviewed by a human before any code)
└── tasks.md          # one task per scenario, each with its four steps
```

Start a feature by copying `_templates/`. The method is described in
`docs/development-process.md`.

## IDs

| Prefix | Meaning | Example |
|---|---|---|
| `US-...` | A user story from the product backlog | `US-SES-01` |
| `FR<n>` | A functional requirement of this feature | `FR2` |
| `NFR<n>` | A non-functional requirement | `NFR1` |
| `T<n>` | A task in `tasks.md` | `T3` |

## Writing requirements (EARS)

Write each requirement as one sentence in one of these forms:

| Pattern | Template |
|---|---|
| Ubiquitous | The system shall ... |
| Event-driven | When <event>, the system shall ... |
| State-driven | While <state>, the system shall ... |
| Unwanted behaviour | If <bad condition>, then the system shall ... |
| Optional feature | Where <feature is included>, the system shall ... |

Name values exactly (status codes, error codes, field names). A requirement is testable if
you can write the scenario that would fail without it.

## Rules

- Resolve open questions in the **Decisions** table before implementation starts.
- Specs change only through a human-reviewed commit, never silently during implementation.
- A requirement with no scenario, or a scenario with no requirement, is a gap to close.

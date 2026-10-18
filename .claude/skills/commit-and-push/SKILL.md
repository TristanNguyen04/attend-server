---
name: commit-and-push
description: Check, commit and push the current branch following this repo's conventions. Use when the user asks to commit, push, or "commit and push" changes.
---

# Commit and push

Never push to `main`. Never force-push unless the user explicitly asks.

1. **Look first.** Run `git status` and `git diff`. Read what will be committed. If there are
   unrelated changes, ask which to include.
2. **Be on a branch.** If the current branch is `main`, create one first, named
   `<type>/<short-description>` with a type such as `feat`, `fix`, `docs`, `test`, `chore`.
3. **Run the checks** and stop if any fails. Do not commit a failing change.
   ```
   cargo fmt --all -- --check
   cargo clippy --all-targets -- -D warnings
   cargo test
   ```
   `cargo test` needs the local database (`docker compose up -d`). If it is not running, say
   so instead of skipping the tests silently.
4. **Stage deliberately.** Add the specific files for this change. Never stage `.env*`,
   `target/`, `bindings/` or editor files.
5. **Write the message.**
   - Subject: imperative, at most 72 characters (`Add session creation endpoint`).
   - Body: why the change is needed, not what the diff shows. Mention the story, requirement
     or task it serves (for example `US-SES-01`, `FR2`, `T3`).
   - End with the attribution line your instructions specify for commits.
6. **Commit, then push** with `git push -u origin <branch>`.
7. **Report** the branch name, the commit hash and which checks ran. Do not open a pull
   request here; use the `create-pull-request` skill for that.

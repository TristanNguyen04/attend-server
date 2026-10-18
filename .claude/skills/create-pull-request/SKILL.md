---
name: create-pull-request
description: Open a pull request for the current branch following this repo's conventions. Use when the user asks to create, open or raise a pull request.
---

# Create a pull request

Do not merge unless the user asks. The base branch is `main`.

1. **Preconditions.** The branch is pushed and is not `main` (use `commit-and-push` first if
   not). Run `git log main..HEAD --oneline` and `git diff main...HEAD --stat` and read them.
   If the checks have not been run on this branch, run them: `cargo fmt --all -- --check`,
   `cargo clippy --all-targets -- -D warnings`, `cargo test`.
2. **Title.** Short and imperative, under 72 characters, describing the change.
3. **Body.** Write it to a temporary file and use these sections:
   - **What and why**: one or two sentences, and the user story or requirement it serves.
   - **Spec and scenarios**: links to `specs/<module>/<feature>/` and the `.feature` files.
   - **How it was tested**: which BDD scenarios and TDD tests cover it, and the result.
   - **Checklist**:
     - [ ] `cargo fmt`, `cargo clippy -D warnings` and `cargo test` pass
     - [ ] Specs and feature files are updated and agree with the code
     - [ ] No secrets; migrations are backward compatible
     - [ ] Module boundaries respected (the architecture test passes)
   - End the body with the attribution line your instructions specify for pull requests.
4. **Create it.**
   ```
   gh pr create --base main --head <branch> --title "<title>" --body-file <file>
   ```
5. **Watch CI** with `gh pr checks --watch`. Report the URL, the check results, and whether
   the pull request is waiting for a review. If a check fails, say which and why. Do not
   hide it.

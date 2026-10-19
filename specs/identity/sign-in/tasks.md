# Tasks: Sign in

Derived from `requirements.md` and `tests/features/identity/sign_in.feature`.
A short list of work items; each names the requirements and scenarios it covers. Built in one
go: code, step definitions and unit tests together (see `docs/development-process.md`).

---

## T1: Sign in with valid credentials  ·  FR1, FR2, FR8, NFR3, NFR4
Scenarios: *Alex signs in with valid credentials*

Groundwork that lands here because this is the first item that needs it:
- `auth.jwt` in `config/{development,test,production}.yaml` (production reads `JWT_SECRET`, no
  default).
- The `Clock` available to the app (`SystemClock` in production; the Cucumber harness puts its
  `FixedClock` in the same place).
- A per-scenario database reset in the Cucumber harness.
- `jsonwebtoken` as a direct dependency; tokens are issued and checked against the `Clock`.
- Account steps create users with SQL and `loco_rs::hash::hash_password`; they never touch the
  module's private entities.
- `POST /api/auth/sign-in`.

## T2: Profile with the access token  ·  FR9, FR8
Scenarios: *Alex views their own profile with the access token*

- An authentication extractor (bearer header, signature check, account lookup) and
  `GET /api/me`, returning exactly `id`, `email`, `full_name`, `status`.

## T3: Canonical email  ·  FR5 (Q5)
Scenarios: *Alex's email is matched regardless of case and surrounding spaces*

- An email normaliser (trim, lower case) used at lookup.
- A forward-only migration adding a `CHECK` that stores emails in the same canonical form, with
  a schema test. Closes Q9 of `specs/platform/persistence`.

## T4: Sign-in failures  ·  FR3, FR4, FR6, FR7, NFR1, NFR2
Scenarios: *A wrong password is rejected*, *An unknown email is rejected exactly like a wrong
password*, *A missing or blank email or password is rejected*, *A correct password on an
account that is not active is refused*, *A wrong password on a suspended account does not
reveal its status*

- Same 401 `invalid_credentials` for unknown email and wrong password, with the same
  password-hash work for both (NFR2); 400 `validation_failed` naming the fields, including for
  a malformed body; 403 `account_not_active` only after the password matches.

## T5: Token failures  ·  FR10, FR11, FR12, FR13, NFR4
Scenarios: *A request without an access token is rejected*, *A malformed or altered access
token is rejected*, *An expired access token is rejected*, *An access token stops working when
the account is suspended*

- The extractor answers with the `ApiError` envelope: `unauthenticated`, `invalid_token`,
  `token_expired` (checked against the `Clock`), `account_not_active`.

---

## Cross-cutting notes

- Done when fmt, clippy, all tests and the coverage gate are green, `docs/architecture.md`
  describes the `auth` configuration, and the pull request says that `shared_lib` gained
  `ApiError::unauthorized` and `ApiError::internal` (shared code needs every team's review).
- `@FR` tags in the feature file link each scenario to its requirement.
- After the T3 migration, `entities/` is unchanged (a `CHECK` does not change the generated
  entities).

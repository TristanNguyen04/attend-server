# Feature: Sign in

Module: `identity`
Stories: A1, A1.1

## User story

> **As a** user of the attendance system,
> **I want** to sign in with my email and password,
> **so that** I can use the attendance features available to me.

Scope note: this is the backend. The user story's screens ("the sign-in page", "my home page")
belong to the mobile and web apps. Here "signed in" means the client holds a valid access token,
and "my home page" is the profile the API returns for that token. SUTD SSO and two-factor
authentication are out of scope for the prototype.

## Definitions

- **Account**: a row of `identity.users`. Its `status` is `INVITED`, `ACTIVE` or `SUSPENDED`.
- **Email**: the login identifier. Stored and compared in canonical form: surrounding whitespace
  removed and lower case.
- **Access token**: a signed JWT (HS256) that a client sends as `Authorization: Bearer <token>`
  on later requests. Its subject is the account id. It carries no email, password or hash.
- **Lifetime**: how long an access token stays valid after it is issued, in seconds. Configured
  as `auth.jwt.expiration`; 86 400 (24 hours) unless a decision below says otherwise.
- **Now**: the time given by the injected `Clock` (`shared_lib::clock`), in UTC.
- **Protected endpoint**: an endpoint that needs a valid access token. In this feature that is
  `GET /api/me`, which returns the caller's own profile.

## Endpoints

| Endpoint | Request | Success |
|---|---|---|
| `POST /api/auth/sign-in` | JSON `{ "email", "password" }` | `200` with `{ "access_token", "token_type": "Bearer", "expires_in" }` |
| `GET /api/me` | `Authorization: Bearer <token>` | `200` with `{ "id", "email", "full_name", "status" }` |

Every error uses the `ApiError` envelope `{ code, message, details }`.

## Functional requirements (EARS)

**FR1: Sign in (event-driven).**
When a client posts the email and password of an `ACTIVE` account and they match, the system
shall respond `200` with an access token, `token_type` `"Bearer"` and `expires_in` equal to the
lifetime.

**FR2: Token content (ubiquitous).**
The system shall issue each access token with the account id as subject, an issue time equal to
now, and an expiry equal to now plus the lifetime, signed with the server secret.

**FR3: Wrong password (unwanted behaviour).**
If the password does not match the account's stored hash (comparison is case-sensitive), then
the system shall respond `401` with code `invalid_credentials` and message
`Invalid email or password`, and shall issue no token.

**FR4: Unknown email (unwanted behaviour).**
If no account has the given email, then the system shall respond exactly as in FR3, so that a
client cannot learn which emails have accounts.

**FR5: Email matching (ubiquitous).**
The system shall match the email after removing surrounding whitespace and lower-casing it, and
shall store every account's email in that canonical form.

**FR6: Missing input (unwanted behaviour).**
If the email or the password is missing, empty or only whitespace, then the system shall respond
`400` with code `validation_failed` and `details` naming each offending field, and shall not look
up any account.

**FR7: Inactive account (unwanted behaviour).**
If the email and password match an account whose status is `INVITED` or `SUSPENDED`, then the
system shall respond `403` with code `account_not_active` and issue no token. If the password
does not match, FR3 applies, so the status of an account is never revealed without its password.

**FR8: Password handling (ubiquitous).**
The system shall store only an Argon2 hash of a password, and shall never return, log or put in a
token a password or a hash.

**FR9: Authenticated request (event-driven).**
When a request to a protected endpoint carries a valid, unexpired access token of an `ACTIVE`
account, the system shall serve it as that account. `GET /api/me` shall return the account's `id`,
`email`, `full_name` and `status`, and nothing else.

**FR10: No token (unwanted behaviour).**
If a request to a protected endpoint has no `Authorization: Bearer` header, then the system shall
respond `401` with code `unauthenticated`.

**FR11: Bad token (unwanted behaviour).**
If the access token is malformed, has a bad signature, or has been altered, then the system shall
respond `401` with code `invalid_token`.

**FR12: Expired token (unwanted behaviour).**
If the access token's expiry is at or before now, then the system shall respond `401` with code
`token_expired`.

**FR13: Account no longer active (unwanted behaviour).**
If a valid access token belongs to an account whose status is no longer `ACTIVE`, then the system
shall respond `403` with code `account_not_active`.

## Non-functional requirements

**NFR1: Error envelope.**
Every error response of this feature has the `ApiError` body `{ code, message, details }` and one
of the codes above. No framework default error body may reach a client (this includes malformed
JSON and a missing body, which are `validation_failed`).

**NFR2: No account enumeration by timing.**
When the email is unknown the system shall still do the same password-hash work as for a known
email, so response time does not reveal whether an account exists.

**NFR3: Secrets.**
The signing secret comes from configuration: a fixed development value in `development` and
`test`, and in `production` only from the environment variable `JWT_SECRET`, with no default.
The secret never appears in logs or responses.

**NFR4: Time.**
Issuing and checking tokens use the injected `Clock`. No test sleeps; scenarios move time with
`FixedClock`.

**NFR5: Coverage.**
Every line of the code this feature adds is covered by a test (the 100% line-coverage gate).

## Decisions (resolved open questions)

"Decided" means the team decided it. "Proposed" is a default from industry practice that nobody has
objected to yet; change it before implementation reaches the task that depends on it.

| # | Question | Decision |
|---|---|---|
| Q1 | What is the login identifier? | **Decided:** the existing `email`. No `username` column. |
| Q2 | How is the professor's UI-worded scenario adapted? | **Decided:** reworded for the API. Page-navigation steps ("on the sign-in page", "see my home page") are dropped; they belong to the app teams' tests. |
| Q3 | How are later requests authenticated? | **Decided:** a signed token on every authenticated request. **Proposed format:** JWT HS256 in `Authorization: Bearer`. |
| Q4 | Token lifetime and refresh | **Decided (2026-10-19):** 24 hours, configurable, no refresh token and no server-side revocation. A user signs in again when it expires. |
| Q5 | Email case and whitespace (this also closes Q9 of `specs/platform/persistence`) | **Decided (2026-10-19):** canonical form is trimmed and lower case, enforced twice: by the code, and by a forward-only migration that adds `CHECK (email = lower(btrim(email)))` to `identity.users` (the table is empty today). |
| Q6 | Response for `INVITED` / `SUSPENDED` | **Decided (2026-10-19):** `403 account_not_active`, but only after the password is verified (FR7). The alternative is the same `401 invalid_credentials` for everyone, which leaks less but gives a suspended user no explanation. |
| Q7 | Status for missing or blank fields | **Proposed:** `400 validation_failed` (the `ApiError` envelope already has a `bad_request` helper). |
| Q8 | Endpoint paths | **Decided (2026-10-19):** `POST /api/auth/sign-in` and `GET /api/me`. Paths belong to the product, not to the module that serves them. |
| Q9 | Time source for tokens | **Proposed:** the injected `Clock`. Loco's `JWT::generate_token` reads the system time, so the module encodes and checks the claims itself with `jsonwebtoken` (a library Loco already uses) and `Clock`. This lets a scenario say "25 hours pass". |
| Q10 | Out of scope | **Proposed:** sign-out, refresh tokens, revocation, password reset, registration and invitation flow, rate limiting and lockout, the device-signed tap, SSO, two-factor. Each needs its own spec. |
| Q11 | A new `ApiError::unauthorized` constructor | **Note:** it lives in `shared_lib`, which needs review from every team. It is a small addition, not a change. |

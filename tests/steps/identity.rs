//! Steps of the `identity` features. Accounts are created with SQL through the test database
//! connection, never through the module's private entities.

// Cucumber hands every `{string}` capture to a step by value, and `{word}` captures that a step
// does not need are named with a leading underscore.
#![allow(clippy::needless_pass_by_value, clippy::used_underscore_binding)]

use attend_server::shared_lib::clock::Clock;
use chrono::Duration;
use cucumber::{given, then, when};
use loco_rs::hash::hash_password;
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use serde_json::json;
use uuid::Uuid;

use super::common::{expand_space, get_with_bearer, post_json};
use crate::AppWorld;

/// The account the Background of every sign-in scenario creates.
const ALEX_EMAIL: &str = "alex@example.edu";
const ALEX_PASSWORD: &str = "Correct#123";

/// Creates an account and remembers its id under its email.
async fn create_account(
    world: &mut AppWorld,
    email: &str,
    name: &str,
    password: &str,
    status: &str,
) {
    let password_hash = hash_password(password).expect("failed to hash the password");
    let row = world
        .ctx
        .db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO identity.users (email, full_name, password_hash, status)
             VALUES ($1, $2, $3, $4) RETURNING id::text",
            [
                email.into(),
                name.into(),
                password_hash.into(),
                status.into(),
            ],
        ))
        .await
        .expect("failed to create the account")
        .expect("the insert returns the new row");
    let id = row
        .try_get_by_index::<String>(0)
        .expect("the id is returned as text");
    world
        .accounts
        .insert(email.to_string(), id.parse().expect("the id is a UUID"));
}

#[given(expr = "an active account exists for {string} named {string} with password {string}")]
async fn an_active_account_exists(
    world: &mut AppWorld,
    email: String,
    name: String,
    password: String,
) {
    create_account(world, &email, &name, &password, "ACTIVE").await;
}

#[given(expr = "an account exists for {string} with password {string} and status {string}")]
async fn an_account_exists(world: &mut AppWorld, email: String, password: String, status: String) {
    create_account(world, &email, "Test User", &password, &status).await;
}

#[when(expr = "{word} signs in with email {string} and password {string}")]
async fn someone_signs_in(world: &mut AppWorld, _who: String, email: String, password: String) {
    let body = json!({
        "email": expand_space(&email),
        "password": expand_space(&password),
    });
    post_json(world, "/api/auth/sign-in", &body).await;
}

#[then("the response contains an access token that expires in 24 hours")]
fn the_response_contains_an_access_token(world: &mut AppWorld) {
    let body = world
        .last_json
        .as_ref()
        .expect("the response has a JSON body");
    assert_eq!(body["token_type"], "Bearer");
    assert_eq!(body["expires_in"], 24 * 60 * 60);

    let token = body["access_token"]
        .as_str()
        .expect("the response has an access token");
    let claims = jsonwebtoken::dangerous::insecure_decode::<serde_json::Value>(token)
        .expect("the access token is a JWT")
        .claims;
    let issued_at = world.clock.now().timestamp();
    assert_eq!(claims["iat"], issued_at, "issued now");
    assert_eq!(
        claims["exp"],
        issued_at + 24 * 60 * 60,
        "expires 24 hours later"
    );
    let subject: Uuid = claims["sub"]
        .as_str()
        .and_then(|sub| sub.parse().ok())
        .expect("the subject is an account id");
    assert!(
        world.accounts.values().any(|id| *id == subject),
        "the subject {subject} is not one of the accounts of this scenario"
    );
}

#[given("Alex has signed in")]
async fn alex_has_signed_in(world: &mut AppWorld) {
    let body = json!({ "email": ALEX_EMAIL, "password": ALEX_PASSWORD });
    post_json(world, "/api/auth/sign-in", &body).await;
    assert_eq!(world.last_status, Some(200), "Alex could not sign in");
    let token = world
        .last_json
        .as_ref()
        .and_then(|body| body["access_token"].as_str());
    world.access_token = Some(
        token
            .expect("the sign-in response has an access token")
            .to_string(),
    );
}

#[when("Alex requests their profile")]
async fn alex_requests_their_profile(world: &mut AppWorld) {
    let token = world.access_token.clone().expect("Alex has signed in");
    get_with_bearer(world, "/api/me", &token).await;
}

#[then(expr = "the profile shows the email {string} and the name {string}")]
fn the_profile_shows(world: &mut AppWorld, email: String, name: String) {
    let profile = world
        .last_json
        .as_ref()
        .expect("the response has a JSON body");
    assert_eq!(profile["email"], email);
    assert_eq!(profile["full_name"], name);
    assert_eq!(profile["status"], "ACTIVE");
    assert_eq!(
        profile["id"],
        world.accounts[&email].to_string(),
        "the profile is the account's own"
    );
}

#[then("the profile does not contain a password or a password hash")]
fn the_profile_has_no_password(world: &mut AppWorld) {
    let profile = world.last_json.as_ref().and_then(|body| body.as_object());
    let profile = profile.expect("the profile is a JSON object");
    let mut keys: Vec<_> = profile.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(keys, ["email", "full_name", "id", "status"]);
    let text = serde_json::Value::Object(profile.clone()).to_string();
    assert!(
        !text.contains(ALEX_PASSWORD),
        "the password is in the profile"
    );
    assert!(
        !text.contains("argon2"),
        "a password hash is in the profile"
    );
}

#[then(expr = "the error code is {string}")]
fn the_error_code_is(world: &mut AppWorld, code: String) {
    let body = world
        .last_json
        .as_ref()
        .expect("the response has a JSON body");
    assert_eq!(body["code"], code);
}

#[then(expr = "the error message is {string}")]
fn the_error_message_is(world: &mut AppWorld, message: String) {
    let body = world
        .last_json
        .as_ref()
        .expect("the response has a JSON body");
    assert_eq!(body["message"], message);
}

#[then("the response contains no access token")]
fn the_response_contains_no_access_token(world: &mut AppWorld) {
    let body = world
        .last_json
        .as_ref()
        .expect("the response has a JSON body");
    assert!(
        body.get("access_token").is_none(),
        "an access token was issued: {body}"
    );
}

#[then(expr = "the error details name the field {string}")]
fn the_error_details_name_the_field(world: &mut AppWorld, field: String) {
    let body = world
        .last_json
        .as_ref()
        .expect("the response has a JSON body");
    let fields = body["details"]["fields"]
        .as_array()
        .unwrap_or_else(|| panic!("the details list no fields: {body}"));
    assert!(
        fields.iter().any(|name| *name == field),
        "{field} is not in {fields:?}"
    );
}

#[when("someone requests a profile without an access token")]
async fn someone_requests_a_profile_without_a_token(world: &mut AppWorld) {
    let response = world.server.get("/api/me").await;
    world.last_status = Some(response.status_code().as_u16());
    world.last_json = serde_json::from_slice(response.as_bytes()).ok();
}

/// The same token with the first character of its signature changed.
fn alter_signature(token: &str) -> String {
    let (head, signature) = token.rsplit_once('.').expect("a JWT has a signature");
    let first = if signature.starts_with('A') { 'B' } else { 'A' };
    format!("{head}.{first}{}", &signature[1..])
}

#[when(expr = "Alex requests their profile with a {word} access token")]
async fn alex_requests_their_profile_with_a_bad_token(world: &mut AppWorld, kind: String) {
    let token = match kind.as_str() {
        "malformed" => "this-is-not-a-jwt".to_string(),
        "altered" => alter_signature(world.access_token.as_deref().expect("Alex has signed in")),
        other => panic!("unknown kind of access token: {other}"),
    };
    get_with_bearer(world, "/api/me", &token).await;
}

#[given(expr = "{int} hours have passed")]
fn hours_have_passed(world: &mut AppWorld, hours: i64) {
    world.clock.advance(Duration::hours(hours));
}

#[given("Alex's account has been suspended")]
async fn alexs_account_has_been_suspended(world: &mut AppWorld) {
    world
        .ctx
        .db
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "UPDATE identity.users SET status = 'SUSPENDED' WHERE id = $1::uuid",
            [world.accounts[ALEX_EMAIL].to_string().into()],
        ))
        .await
        .expect("failed to suspend the account");
}

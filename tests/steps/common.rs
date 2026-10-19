//! Steps shared by all features.

use cucumber::{given, then, when};

use crate::AppWorld;

#[given("the API is running")]
fn the_api_is_running(_world: &mut AppWorld) {
    // The app is booted once for the whole run (see `booted` in tests/cucumber.rs).
}

/// `When a client requests "GET /_ping"`: the method and the path, separated by a space.
#[when(expr = "a client requests {string}")]
async fn a_client_requests(world: &mut AppWorld, request: String) {
    let (method, path) = request
        .split_once(' ')
        .unwrap_or_else(|| panic!("expected \"<METHOD> <path>\", got {request:?}"));
    let response = match method {
        "GET" => world.server.get(path).await,
        other => panic!("method {other} is not supported by this step yet"),
    };
    world.last_status = Some(response.status_code().as_u16());
    world.last_json = serde_json::from_slice(response.as_bytes()).ok();
}

/// Sends `body` as JSON to `path` and records the status and JSON of the response.
pub async fn post_json(world: &mut AppWorld, path: &str, body: &serde_json::Value) {
    let response = world.server.post(path).json(body).await;
    world.last_status = Some(response.status_code().as_u16());
    world.last_json = serde_json::from_slice(response.as_bytes()).ok();
}

/// Sends a GET with `Authorization: Bearer <token>` and records the status and JSON of the
/// response.
pub async fn get_with_bearer(world: &mut AppWorld, path: &str, token: &str) {
    let response = world
        .server
        .get(path)
        .add_header("Authorization", format!("Bearer {token}"))
        .await;
    world.last_status = Some(response.status_code().as_u16());
    world.last_json = serde_json::from_slice(response.as_bytes()).ok();
}

/// In Gherkin tables `{space}` stands for one space, because table cells are trimmed.
pub fn expand_space(cell: &str) -> String {
    cell.replace("{space}", " ")
}

#[then(expr = "the response status is {int}")]
fn the_response_status_is(world: &mut AppWorld, expected: u16) {
    assert_eq!(
        world.last_status,
        Some(expected),
        "expected HTTP {expected} but got {:?}",
        world.last_status
    );
}

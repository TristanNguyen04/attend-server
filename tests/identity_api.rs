//! HTTP-level checks of the identity API that no scenario states: bodies the framework would
//! reject with its own error format must come back in the `ApiError` envelope (NFR1).

use attend_server::app::App;
use axum::body::Bytes;
use loco_rs::{testing::prelude::boot_test, TestServer};
use serde_json::{json, Value};
use serial_test::serial;

async fn server() -> TestServer {
    let boot = boot_test::<App>()
        .await
        .expect("failed to boot the test app");
    let router = boot.router.expect("the boot result includes a router");
    TestServer::new(router.into_make_service_with_connect_info::<std::net::SocketAddr>())
        .expect("failed to start the test server")
}

/// Posts `body` to the sign-in endpoint and returns the status and JSON of the response.
async fn sign_in(server: &TestServer, body: &str, content_type: Option<&str>) -> (u16, Value) {
    let mut request = server
        .post("/api/auth/sign-in")
        .bytes(Bytes::from(body.to_string()));
    if let Some(content_type) = content_type {
        request = request.content_type(content_type);
    }
    let response = request.await;
    (
        response.status_code().as_u16(),
        serde_json::from_slice(response.as_bytes()).expect("a JSON error body"),
    )
}

#[tokio::test]
#[serial]
async fn a_sign_in_body_that_is_not_valid_is_a_validation_failure() {
    let server = server().await;
    let json = Some("application/json");
    let bodies = [
        ("{", json),
        ("", json),
        ("{}", None),
        (r#"{"email": 5, "password": "x"}"#, json),
    ];
    for (body, content_type) in bodies {
        let (status, error) = sign_in(&server, body, content_type).await;
        assert_eq!(status, 400, "{body:?}: {error}");
        assert_eq!(error["code"], "validation_failed", "{body:?}: {error}");
        assert_eq!(error["details"], json!({ "fields": ["body"] }), "{body:?}");
    }
}

#[tokio::test]
#[serial]
async fn an_empty_json_object_names_both_fields() {
    let server = server().await;
    let (status, error) = sign_in(&server, "{}", Some("application/json")).await;
    assert_eq!(status, 400);
    assert_eq!(error["details"], json!({ "fields": ["email", "password"] }));
}

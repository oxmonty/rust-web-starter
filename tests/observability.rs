mod common;

use common::{ErrorBody, spawn_app};

const REQUEST_ID: &str = "x-request-id";

#[tokio::test]
async fn response_carries_a_generated_request_id() {
    // given: a running app and a request that sets no id of its own
    let app = spawn_app().await;

    // when: calling any endpoint
    let response = app
        .client
        .get(app.url("/healthz"))
        .send()
        .await
        .expect("send healthz");

    // then: the service generated one and echoed it back
    assert_eq!(response.status(), 200);
    let request_id = response
        .headers()
        .get(REQUEST_ID)
        .expect("x-request-id header");
    assert!(!request_id.is_empty());
}

#[tokio::test]
async fn caller_supplied_request_id_is_preserved() {
    // given: a running app and a caller that already has a trace id
    let app = spawn_app().await;

    // when: sending it as x-request-id
    let response = app
        .client
        .get(app.url("/healthz"))
        .header(REQUEST_ID, "caller-supplied-id")
        .send()
        .await
        .expect("send healthz");

    // then: it survives instead of being replaced, so both sides log the same id
    assert_eq!(
        response.headers().get(REQUEST_ID).unwrap(),
        "caller-supplied-id"
    );
}

// the panic guard is a layer, not a route, so it is exercised against a handler that panics
// on purpose. The panic message cargo prints during this test is expected.
#[tokio::test]
async fn panicking_handler_returns_500_instead_of_dropping_the_connection() {
    // given: a route that panics, behind the layer the app uses
    // a concrete return type, because a handler returning `!` is not a Handler
    async fn boom() -> &'static str {
        panic!("handler exploded")
    }

    let router = axum::Router::new()
        .route("/boom", axum::routing::get(boom))
        .layer(rust_web_starter::telemetry::catch_panic_layer());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let addr = listener.local_addr().expect("local addr");
    tokio::spawn(async move { axum::serve(listener, router).await.expect("server error") });

    // when: calling it
    let response = reqwest::Client::new()
        .get(format!("http://{addr}/boom"))
        .send()
        .await
        .expect("send boom");

    // then: the caller gets the standard error body, not a dropped connection
    assert_eq!(response.status(), 500);
    let body: ErrorBody = response.json().await.expect("parse error body");
    assert_eq!(body.error, "internal_error");
    assert_eq!(body.message, "An internal error occurred.");
}

mod common;

use common::{ErrorBody, Todo, spawn_app};
use serde_json::json;

#[tokio::test]
async fn full_todo_lifecycle() {
    // given: a running app and a new todo payload
    let app = spawn_app().await;

    // when: creating the todo
    let response = app
        .client
        .post(app.url("/todos"))
        .json(&json!({ "title": "write tests", "description": "cover the api" }))
        .send()
        .await
        .expect("send create");
    // then: it is created with the submitted fields and done = false
    assert_eq!(response.status(), 201);
    let created: Todo = response.json().await.expect("parse created todo");
    assert_eq!(created.title, "write tests");
    assert_eq!(created.description.as_deref(), Some("cover the api"));
    assert!(!created.done);

    // when: reading it back
    let response = app
        .client
        .get(app.url(&format!("/todos/{}", created.id)))
        .send()
        .await
        .expect("send get");
    // then: it matches what was created
    assert_eq!(response.status(), 200);
    let fetched: Todo = response.json().await.expect("parse fetched todo");
    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.title, "write tests");
    assert_eq!(fetched.created_at, created.created_at);

    // when: updating it
    let response = app
        .client
        .put(app.url(&format!("/todos/{}", created.id)))
        .json(&json!({ "title": "write more tests", "description": null, "done": true }))
        .send()
        .await
        .expect("send update");
    // then: the fields reflect the update
    assert_eq!(response.status(), 200);
    let updated: Todo = response.json().await.expect("parse updated todo");
    assert_eq!(updated.title, "write more tests");
    assert_eq!(updated.description, None);
    assert!(updated.done);

    // when: deleting it
    let response = app
        .client
        .delete(app.url(&format!("/todos/{}", created.id)))
        .send()
        .await
        .expect("send delete");
    // then: it is gone, and re-reading returns 404
    assert_eq!(response.status(), 204);
    let response = app
        .client
        .get(app.url(&format!("/todos/{}", created.id)))
        .send()
        .await
        .expect("send get after delete");
    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn get_unknown_todo_returns_404_with_error_body() {
    // given: a running app and an id that was never created
    let app = spawn_app().await;

    // when: fetching that id
    let response = app
        .client
        .get(app.url("/todos/2000000000"))
        .send()
        .await
        .expect("send get");

    // then: 404 with a well-formed error body
    assert_eq!(response.status(), 404);
    let body: ErrorBody = response.json().await.expect("parse error body");
    assert_eq!(body.error, "not_found");
    assert!(!body.message.is_empty());
}

#[tokio::test]
async fn create_with_empty_title_returns_422() {
    // given: a running app
    let app = spawn_app().await;

    // when: creating a todo with an empty title
    let response = app
        .client
        .post(app.url("/todos"))
        .json(&json!({ "title": "", "description": null }))
        .send()
        .await
        .expect("send create");

    // then: validation fails
    assert_eq!(response.status(), 422);
}

#[tokio::test]
async fn create_with_title_over_200_chars_returns_422() {
    // given: a running app and a title over the 200 char limit
    let app = spawn_app().await;
    let title = "a".repeat(201);

    // when: creating a todo with that title
    let response = app
        .client
        .post(app.url("/todos"))
        .json(&json!({ "title": title, "description": null }))
        .send()
        .await
        .expect("send create");

    // then: validation fails
    assert_eq!(response.status(), 422);
}

#[tokio::test]
async fn list_with_limit_over_100_returns_422() {
    // given: a running app
    let app = spawn_app().await;

    // when: listing with limit=500
    let response = app
        .client
        .get(app.url("/todos?limit=500"))
        .send()
        .await
        .expect("send list");

    // then: validation fails
    assert_eq!(response.status(), 422);
}

#[tokio::test]
async fn list_with_negative_offset_returns_422() {
    // given: a running app
    let app = spawn_app().await;

    // when: listing with a negative offset
    let response = app
        .client
        .get(app.url("/todos?offset=-1"))
        .send()
        .await
        .expect("send list");

    // then: validation fails
    assert_eq!(response.status(), 422);
}

#[tokio::test]
async fn list_with_limit_one_returns_at_most_one() {
    // given: a running app with at least one todo present
    let app = spawn_app().await;
    app.client
        .post(app.url("/todos"))
        .json(&json!({ "title": "pagination seed", "description": null }))
        .send()
        .await
        .expect("send create");

    // when: listing with limit=1
    let response = app
        .client
        .get(app.url("/todos?limit=1"))
        .send()
        .await
        .expect("send list");

    // then: at most one item comes back
    assert_eq!(response.status(), 200);
    let todos: Vec<Todo> = response.json().await.expect("parse todos");
    assert!(todos.len() <= 1);
}

#[tokio::test]
async fn list_contains_created_todo() {
    // given: a running app and a freshly created todo
    let app = spawn_app().await;
    let response = app
        .client
        .post(app.url("/todos"))
        .json(&json!({ "title": "findable todo", "description": null }))
        .send()
        .await
        .expect("send create");
    let created: Todo = response.json().await.expect("parse created todo");

    // when: listing todos
    let response = app
        .client
        .get(app.url("/todos?limit=100&offset=0"))
        .send()
        .await
        .expect("send list");

    // then: the created todo is present in the response
    assert_eq!(response.status(), 200);
    let todos: Vec<Todo> = response.json().await.expect("parse todos");
    assert!(todos.iter().any(|todo| todo.id == created.id));
}

#[tokio::test]
async fn healthz_returns_200_without_touching_the_database() {
    // given: a running app
    let app = spawn_app().await;

    // when: hitting healthz
    let response = app
        .client
        .get(app.url("/healthz"))
        .send()
        .await
        .expect("send healthz");

    // then: it reports ok
    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("parse body");
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn readyz_returns_200_when_db_is_up() {
    // given: a running app with a working db pool
    let app = spawn_app().await;

    // when: hitting readyz
    let response = app
        .client
        .get(app.url("/readyz"))
        .send()
        .await
        .expect("send readyz");

    // then: it reports ready
    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("parse body");
    assert_eq!(body["status"], "ready");
}

#[tokio::test]
async fn update_bumps_updated_at() {
    // given: a created todo
    let app = spawn_app().await;
    let response = app
        .client
        .post(app.url("/todos"))
        .json(&json!({ "title": "timestamp check", "description": null }))
        .send()
        .await
        .expect("send create");
    let created: Todo = response.json().await.expect("parse created todo");

    // sqlite's CURRENT_TIMESTAMP default has 1-second resolution; wait past it so the
    // update's updated_at is unambiguously later than the insert's.
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

    // when: updating the todo
    let response = app
        .client
        .put(app.url(&format!("/todos/{}", created.id)))
        .json(&json!({ "title": "timestamp check", "description": null, "done": true }))
        .send()
        .await
        .expect("send update");

    // then: updated_at moved forward
    assert_eq!(response.status(), 200);
    let updated: Todo = response.json().await.expect("parse updated todo");
    assert!(updated.updated_at > created.updated_at);
}

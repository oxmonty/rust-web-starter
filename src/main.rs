use axum::{
    extract::{Query, State},
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use rand::Rng;
use std::collections::HashMap;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[derive(OpenApi)]
#[openapi(
    info(title = "Hello World API", description = "A Rust + Axum demo with Swagger"),
    paths(home, counter, game)
)]
struct ApiDoc;

#[derive(Clone)]
struct AppState {
    counter: Arc<AtomicU64>,
    target: Arc<Mutex<u32>>,
}

#[tokio::main]
async fn main() {
    let state = AppState {
        counter: Arc::new(AtomicU64::new(0)),
        target: Arc::new(Mutex::new(rand::thread_rng().gen_range(1..=20))),
    };

    let app = Router::new()
        .route("/", get(home))
        .route("/counter", get(counter))
        .route("/game", get(game))
        .merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:4563").await.unwrap();
    println!("Serving at http://localhost:4563");
    println!("Swagger UI at http://localhost:4563/docs");
    axum::serve(listener, app).await.unwrap();
}

fn nav() -> &'static str {
    r#"<nav style="margin-bottom:1em;">
  <a href="/">Home</a> |
  <a href="/counter">Counter</a> |
  <a href="/game">Number Guessing Game</a> |
  <a href="/docs">API Docs</a>
</nav>"#
}

#[utoipa::path(
    get,
    path = "/",
    responses(
        (status = 200, description = "Home page", content_type = "text/html")
    )
)]
async fn home() -> Html<String> {
    let nav_html = nav();
    Html(format!(
        r#"<!DOCTYPE html>
<html><head><title>Hello, world!</title></head>
<body>
{nav_html}
<h1>Hello, world!</h1>
<p>Welcome to the Rust + Axum demo.</p>
</body></html>"#
    ))
}

#[utoipa::path(
    get,
    path = "/counter",
    responses(
        (status = 200, description = "Visit counter page", content_type = "text/html")
    )
)]
async fn counter(State(state): State<AppState>) -> Html<String> {
    let hits = state.counter.fetch_add(1, Ordering::SeqCst) + 1;
    let nav_html = nav();
    Html(format!(
        r#"<!DOCTYPE html>
<html><head><title>Counter</title></head>
<body>
{nav_html}
<h1>Visit Counter</h1>
<p>This page has been visited <strong>{hits}</strong> times.</p>
<p><a href="/counter">Refresh</a> to watch it go up.</p>
</body></html>"#
    ))
}

#[utoipa::path(
    get,
    path = "/game",
    params(
        ("guess" = Option<u32>, Query, description = "Your guess between 1 and 20")
    ),
    responses(
        (status = 200, description = "Number guessing game page", content_type = "text/html")
    )
)]
async fn game(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let guess = params.get("guess").and_then(|g| g.parse::<u32>().ok());

    let mut target = state.target.lock().unwrap();

    let message = match guess {
        None => {
            *target = rand::thread_rng().gen_range(1..=20);
            String::from("I'm thinking of a number between 1 and 20. Make a guess!")
        }
        Some(n) if n < *target => format!("{n} is too low. Try higher."),
        Some(n) if n > *target => format!("{n} is too high. Try lower."),
        Some(_) => {
            let answer = *target;
            *target = rand::thread_rng().gen_range(1..=20);
            format!("{answer} is correct! Well done! I've picked a new number — guess again.")
        }
    };

    let nav_html = nav();
    Html(format!(
        r#"<!DOCTYPE html>
<html><head><title>Number Guessing Game</title></head>
<body>
{nav_html}
<h1>Number Guessing Game</h1>
<p>{message}</p>
<form action="/game" method="get">
  <input type="number" name="guess" min="1" max="20" placeholder="1-20" autofocus />
  <button type="submit">Guess</button>
</form>
</body></html>"#
    ))
}

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
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Serving at http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}

fn nav() -> &'static str {
    r#"<nav style="margin-bottom:1em;">
  <a href="/">Home</a> |
  <a href="/counter">Counter</a> |
  <a href="/game">Number Guessing Game</a>
</nav>"#
}

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

async fn game(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let guess = params.get("guess").and_then(|g| g.parse::<u32>().ok());

    let mut target = state.target.lock().unwrap();

    let message = match guess {
        None => {
            // New visit: generate a fresh random target
            *target = rand::thread_rng().gen_range(1..=20);
            String::from("I'm thinking of a number between 1 and 20. Make a guess!")
        }
        Some(n) if n < *target => format!("{n} is too low. Try higher."),
        Some(n) if n > *target => format!("{n} is too high. Try lower."),
        Some(_) => {
            let answer = *target;
            // Generate a new target after a correct guess
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

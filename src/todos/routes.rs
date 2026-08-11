use axum::Json;
use axum::extract::{FromRequestParts, Path, Query, State};
use axum::http::StatusCode;
use axum::http::request::Parts;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::AppState;
use crate::error::{AppError, ErrorBody};

use super::model::{NewTodo, Pagination, Todo, UpdateTodo};
use super::repo;

impl<S> FromRequestParts<S> for Pagination
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(pagination) = Query::<Pagination>::from_request_parts(parts, state)
            .await
            .map_err(|err| AppError::Validation(err.to_string()))?;
        pagination.validate()?;
        Ok(pagination)
    }
}

#[utoipa::path(
    get,
    path = "/todos",
    params(Pagination),
    responses(
        (status = 200, description = "List todos", body = [Todo]),
        (status = 422, description = "Invalid pagination", body = ErrorBody),
    ),
)]
async fn list_todos(
    State(state): State<AppState>,
    pagination: Pagination,
) -> Result<Json<Vec<Todo>>, AppError> {
    let todos = repo::list(&state.pool, pagination).await?;
    Ok(Json(todos))
}

#[utoipa::path(
    post,
    path = "/todos",
    request_body = NewTodo,
    responses(
        (status = 201, description = "Todo created", body = Todo),
        (status = 422, description = "Invalid payload", body = ErrorBody),
    ),
)]
async fn create_todo(
    State(state): State<AppState>,
    Json(payload): Json<NewTodo>,
) -> Result<(StatusCode, Json<Todo>), AppError> {
    let todo = repo::create(&state.pool, payload).await?;
    Ok((StatusCode::CREATED, Json(todo)))
}

#[utoipa::path(
    get,
    path = "/todos/{id}",
    params(("id" = i32, Path, description = "Todo id")),
    responses(
        (status = 200, description = "Todo found", body = Todo),
        (status = 404, description = "Todo not found", body = ErrorBody),
    ),
)]
async fn get_todo(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<Todo>, AppError> {
    let todo = repo::find(&state.pool, id).await?;
    Ok(Json(todo))
}

#[utoipa::path(
    put,
    path = "/todos/{id}",
    params(("id" = i32, Path, description = "Todo id")),
    request_body = UpdateTodo,
    responses(
        (status = 200, description = "Todo updated", body = Todo),
        (status = 404, description = "Todo not found", body = ErrorBody),
        (status = 422, description = "Invalid payload", body = ErrorBody),
    ),
)]
async fn update_todo(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateTodo>,
) -> Result<Json<Todo>, AppError> {
    let todo = repo::update(&state.pool, id, payload).await?;
    Ok(Json(todo))
}

#[utoipa::path(
    delete,
    path = "/todos/{id}",
    params(("id" = i32, Path, description = "Todo id")),
    responses(
        (status = 204, description = "Todo deleted"),
        (status = 404, description = "Todo not found", body = ErrorBody),
    ),
)]
async fn delete_todo(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<StatusCode, AppError> {
    repo::delete(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_todos, create_todo))
        .routes(routes!(get_todo, update_todo, delete_todo))
}

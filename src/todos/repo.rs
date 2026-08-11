use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::db::DbPool;
use crate::db::schema::todos;
use crate::error::AppError;

use super::model::{NewTodo, Pagination, Todo, UpdateTodo};

pub async fn list(pool: &DbPool, pagination: Pagination) -> Result<Vec<Todo>, AppError> {
    let mut conn = pool
        .get()
        .await
        .map_err(|err| AppError::Pool(err.to_string()))?;
    let rows = todos::table
        .order(todos::id.asc())
        .limit(pagination.limit)
        .offset(pagination.offset)
        .select(Todo::as_select())
        .load(&mut conn)
        .await?;
    Ok(rows)
}

pub async fn create(pool: &DbPool, new_todo: NewTodo) -> Result<Todo, AppError> {
    new_todo.validate()?;
    let mut conn = pool
        .get()
        .await
        .map_err(|err| AppError::Pool(err.to_string()))?;
    let todo = diesel::insert_into(todos::table)
        .values(&new_todo)
        .returning(Todo::as_returning())
        .get_result(&mut conn)
        .await?;
    Ok(todo)
}

pub async fn find(pool: &DbPool, id: i32) -> Result<Todo, AppError> {
    let mut conn = pool
        .get()
        .await
        .map_err(|err| AppError::Pool(err.to_string()))?;
    let todo = todos::table
        .find(id)
        .select(Todo::as_select())
        .first(&mut conn)
        .await?;
    Ok(todo)
}

pub async fn update(pool: &DbPool, id: i32, payload: UpdateTodo) -> Result<Todo, AppError> {
    payload.validate()?;
    let mut conn = pool
        .get()
        .await
        .map_err(|err| AppError::Pool(err.to_string()))?;
    let todo = diesel::update(todos::table.find(id))
        .set((
            todos::title.eq(payload.title),
            todos::description.eq(payload.description),
            todos::done.eq(payload.done),
            todos::updated_at.eq(chrono::Utc::now().naive_utc()),
        ))
        .returning(Todo::as_returning())
        .get_result(&mut conn)
        .await?;
    Ok(todo)
}

pub async fn delete(pool: &DbPool, id: i32) -> Result<(), AppError> {
    let mut conn = pool
        .get()
        .await
        .map_err(|err| AppError::Pool(err.to_string()))?;
    let affected = diesel::delete(todos::table.find(id))
        .execute(&mut conn)
        .await?;
    if affected == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

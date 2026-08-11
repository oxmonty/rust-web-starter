use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::db::schema::todos;
use crate::error::AppError;

const MAX_TITLE_LEN: usize = 200;

#[derive(Debug, Clone, Queryable, Selectable, Serialize, ToSchema)]
#[diesel(table_name = todos)]
pub struct Todo {
    pub id: i32,
    pub title: String,
    pub description: Option<String>,
    pub done: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Deserialize, Insertable, ToSchema)]
#[diesel(table_name = todos)]
pub struct NewTodo {
    pub title: String,
    pub description: Option<String>,
}

impl NewTodo {
    pub fn validate(&self) -> Result<(), AppError> {
        validate_title(&self.title)
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateTodo {
    pub title: String,
    pub description: Option<String>,
    pub done: bool,
}

impl UpdateTodo {
    pub fn validate(&self) -> Result<(), AppError> {
        validate_title(&self.title)
    }
}

fn validate_title(title: &str) -> Result<(), AppError> {
    if title.trim().is_empty() {
        return Err(AppError::Validation("title must not be empty".to_string()));
    }
    if title.len() > MAX_TITLE_LEN {
        return Err(AppError::Validation(format!(
            "title must be at most {MAX_TITLE_LEN} characters"
        )));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, Deserialize, IntoParams)]
pub struct Pagination {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    20
}

impl Pagination {
    pub fn validate(&self) -> Result<(), AppError> {
        if self.limit < 0 || self.limit > 100 || self.offset < 0 {
            return Err(AppError::Validation(
                "limit must be between 0 and 100, offset must be >= 0".to_string(),
            ));
        }
        Ok(())
    }
}

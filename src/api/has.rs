use super::db_get_blob;
use actix_web::{http::StatusCode, web, HttpResponse, ResponseError};
use sqlx::SqlitePool;
use tracing::instrument;

#[derive(thiserror::Error, Debug)]
pub enum HasBlobError {
    #[error("file not found")]
    NotFoundError,
    #[error("database error")]
    DbError(#[from] sqlx::Error),
}

impl ResponseError for HasBlobError {
    fn status_code(&self) -> StatusCode {
        match self {
            HasBlobError::NotFoundError => StatusCode::NOT_FOUND,
            HasBlobError::DbError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[instrument(skip(hash, db))]
pub async fn has(
    hash: web::Path<String>,
    db: web::Data<SqlitePool>,
) -> Result<HttpResponse, actix_web::Error> {
    db_get_blob(&db, &hash).await.map_err(|e| match e {
        sqlx::Error::RowNotFound => HasBlobError::NotFoundError,
        _ => HasBlobError::DbError(e),
    })?;

    Ok(HttpResponse::Ok().finish())
}

#[instrument(skip(path, db))]
pub async fn has_with_ext(
    path: web::Path<(String, String)>,
    db: web::Data<SqlitePool>,
) -> Result<HttpResponse, actix_web::Error> {
    db_get_blob(&db, &path.0).await.map_err(|e| match e {
        sqlx::Error::RowNotFound => HasBlobError::NotFoundError,
        _ => HasBlobError::DbError(e),
    })?;

    Ok(HttpResponse::Ok().finish())
}

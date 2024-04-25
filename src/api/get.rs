use actix_web::{http::StatusCode, web, HttpResponse, ResponseError};
use sqlx::SqlitePool;
use tracing::instrument;

pub struct GetBlob {
    pub pubkey: String,
    pub blob: Vec<u8>,
    pub r#type: String,
}

#[derive(thiserror::Error, Debug)]
pub enum GetBlobError {
    #[error("file not found")]
    NotFoundError,
    #[error("database error")]
    DbError(#[from] sqlx::Error),
}

impl ResponseError for GetBlobError {
    fn status_code(&self) -> StatusCode {
        match self {
            GetBlobError::NotFoundError => StatusCode::NOT_FOUND,
            GetBlobError::DbError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[instrument(skip(hash, db))]
pub async fn get(
    hash: web::Path<String>,
    db: web::Data<SqlitePool>,
) -> Result<HttpResponse, actix_web::Error> {
    let blob = db_get_blob(&db, &hash).await.map_err(|e| match e {
        sqlx::Error::RowNotFound => GetBlobError::NotFoundError,
        _ => GetBlobError::DbError(e),
    })?;

    Ok(HttpResponse::Ok()
        .insert_header(("Content-Type", blob.r#type))
        .body(blob.blob))
}

#[instrument(skip(path, db))]
pub async fn get_with_ext(
    path: web::Path<(String, String)>,
    db: web::Data<SqlitePool>,
) -> Result<HttpResponse, actix_web::Error> {
    let blob = db_get_blob(&db, &path.0).await.map_err(|e| match e {
        sqlx::Error::RowNotFound => GetBlobError::NotFoundError,
        _ => GetBlobError::DbError(e),
    })?;

    Ok(HttpResponse::Ok()
        .insert_header(("Content-Type", blob.r#type))
        .body(blob.blob))
}

pub async fn db_get_blob(db: &SqlitePool, hash: &str) -> Result<GetBlob, sqlx::Error> {
    let blob = sqlx::query_as!(
        GetBlob,
        r#"
        SELECT pubkey, blob, type
        FROM blobs
        WHERE hash = $1
        LIMIT 1
    "#,
        hash,
    )
    .fetch_one(db)
    .await?;

    Ok(blob)
}

use actix_web::{http::StatusCode, web, HttpResponse, ResponseError};
use serde::Serialize;
use sqlx::SqlitePool;
use tracing::instrument;

use crate::config::Config;

#[derive(Serialize)]
pub struct BlobDescriptor {
    pub pubkey: String,
    pub hash: String,
    pub url: String,
    pub r#type: String,
    pub size: i64,
    pub created: i64,
}

pub struct DbBlob {
    pub pubkey: String,
    pub hash: String,
    pub r#type: String,
    pub size: i64,
    pub created: i64,
}

#[derive(thiserror::Error, Debug)]
pub enum ListBlobsError {
    #[error("no blobs for pubkey yet")]
    NotFoundError,
    #[error("database error")]
    DbError(#[from] sqlx::Error),
}

impl ResponseError for ListBlobsError {
    fn status_code(&self) -> StatusCode {
        match self {
            ListBlobsError::NotFoundError => StatusCode::NOT_FOUND,
            ListBlobsError::DbError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[instrument(skip(pubkey, db, cfg))]
pub async fn list(
    pubkey: web::Path<String>,
    db: web::Data<SqlitePool>,
    cfg: web::Data<Config>,
) -> Result<HttpResponse, actix_web::Error> {
    let blobs = db_get_blobs(&db, &pubkey).await.map_err(|e| match e {
        sqlx::Error::RowNotFound => ListBlobsError::NotFoundError,
        _ => ListBlobsError::DbError(e),
    })?;

    let full_blobs: Vec<_> = blobs
        .into_iter()
        .map(|b| BlobDescriptor {
            url: format!("{}/{}", cfg.cdn.base_url, b.hash),
            pubkey: b.pubkey,
            hash: b.hash,
            r#type: b.r#type,
            size: b.size,
            created: b.created,
        })
        .collect();

    Ok(HttpResponse::Ok().json(full_blobs))
}

pub async fn db_get_blobs(db: &SqlitePool, pubkey: &str) -> Result<Vec<DbBlob>, sqlx::Error> {
    let blobs = sqlx::query_as!(
        DbBlob,
        r#"
        SELECT pubkey, hash, type, size, created
        FROM blobs
        WHERE pubkey = $1
    "#,
        pubkey,
    )
    .fetch_all(db)
    .await?;

    Ok(blobs)
}

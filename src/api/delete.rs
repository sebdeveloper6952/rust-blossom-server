use actix_web::{
    http::StatusCode,
    web::{Data, Path, ReqData},
    HttpResponse, ResponseError,
};
use sqlx::{sqlite::SqliteQueryResult, SqlitePool};
use tracing::instrument;

use super::db_get_blob;

#[derive(thiserror::Error, Debug)]
pub enum DeleteError {
    #[error("file not found")]
    NotFoundError,
    #[error("database error")]
    DbError(#[from] sqlx::Error),
}

impl ResponseError for DeleteError {
    fn status_code(&self) -> StatusCode {
        match self {
            DeleteError::NotFoundError => StatusCode::NOT_FOUND,
            DeleteError::DbError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[instrument(skip(hash, db))]
pub async fn delete(
    hash: Path<String>,
    pubkey: ReqData<nostr::PublicKey>,
    db: Data<SqlitePool>,
) -> Result<HttpResponse, DeleteError> {
    let blob = db_get_blob(&db, &hash).await?;

    if pubkey.to_string() != blob.pubkey {
        // TODO: forbidden
    }

    db_delete_blob(&db, &hash).await?;

    Ok(HttpResponse::Ok().finish())
}

async fn db_delete_blob(db: &SqlitePool, hash: &str) -> Result<SqliteQueryResult, sqlx::Error> {
    sqlx::query!(r#"DELETE FROM blobs WHERE hash = $1"#, hash,)
        .execute(db)
        .await
}

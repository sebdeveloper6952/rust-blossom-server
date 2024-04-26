use actix_web::{
    http::StatusCode,
    web::{Bytes, Data, ReqData},
    HttpResponse, ResponseError,
};
use chrono::Utc;
use sha256::digest;
use sqlx::{sqlite::SqliteQueryResult, SqlitePool};
use std::convert::TryFrom;
use tracing::instrument;

#[derive(thiserror::Error, Debug)]
pub enum UploadError {
    #[error("failed to insert blob into DB")]
    DbInsertError(#[from] sqlx::Error),
    #[error("failed to extract payload size")]
    ExtractPayloadSizeError,
}

impl ResponseError for UploadError {
    fn status_code(&self) -> StatusCode {
        match self {
            UploadError::DbInsertError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            UploadError::ExtractPayloadSizeError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[instrument(skip(payload, db))]
pub async fn upload(
    pubkey: ReqData<nostr::PublicKey>,
    payload: ReqData<Bytes>,
    db: Data<SqlitePool>,
) -> Result<HttpResponse, UploadError> {
    let payload_size =
        i32::try_from(payload.len()).map_err(|_| UploadError::ExtractPayloadSizeError)?;

    let bytes_vec = payload.into_inner().to_vec();
    let mime_type = match infer::get(&bytes_vec) {
        Some(t) => t.to_string(),
        _ => String::from("application/octet-stream"),
    };
    let hash = digest(&bytes_vec);

    let _ = db_insert_blob(
        &db,
        &pubkey.to_string(),
        &hash,
        &bytes_vec,
        &mime_type,
        payload_size,
    )
    .await?;

    // TODO: must return BlobDescriptor here

    Ok(HttpResponse::Ok()
        .json(serde_json::json!({"size": payload_size, "hash": hash, "type": mime_type})))
}

async fn db_insert_blob(
    db: &SqlitePool,
    pubkey: &str,
    hash: &str,
    bytes_vec: &[u8],
    mime_type: &str,
    payload_size: i32,
) -> Result<SqliteQueryResult, sqlx::Error> {
    let now = Utc::now();
    sqlx::query!(
        r#"
        INSERT INTO blobs (pubkey, hash, blob, type, size, created)
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (hash) DO NOTHING
    "#,
        pubkey,
        hash,
        bytes_vec,
        mime_type,
        payload_size,
        now,
    )
    .execute(db)
    .await
}

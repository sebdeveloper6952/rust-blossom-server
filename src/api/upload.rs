use crate::{
    api::{db_get_blob, GetBlob},
    blossom::BlobDescriptor,
};
use actix_web::{
    http::StatusCode,
    web::{Bytes, Data, ReqData},
    HttpResponse, ResponseError,
};
use chrono::Utc;
use sha256::digest;
use sqlx::SqlitePool;
use std::convert::TryFrom;
use tracing::instrument;

use crate::config::Config;

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

#[instrument(skip(payload, db, cfg))]
pub async fn upload(
    pubkey: ReqData<nostr::PublicKey>,
    payload: ReqData<Bytes>,
    db: Data<SqlitePool>,
    cfg: Data<Config>,
) -> Result<HttpResponse, UploadError> {
    let payload_size =
        i32::try_from(payload.len()).map_err(|_| UploadError::ExtractPayloadSizeError)?;

    let bytes_vec = payload.into_inner().to_vec();
    let mime_type = match infer::get(&bytes_vec) {
        Some(t) => t.to_string(),
        _ => String::from("application/octet-stream"),
    };
    let hash = digest(&bytes_vec);

    match db_get_blob(&db, &hash).await {
        Ok(blob) => {
            return Ok(HttpResponse::Ok().json(BlobDescriptor {
                pubkey: blob.pubkey,
                hash: String::from(&hash),
                url: format!("{}/{}", cfg.cdn.base_url, &hash),
                r#type: blob.r#type,
                size: blob.size,
                created: blob.created,
            }))
        }
        _ => {}
    };

    let blob = db_insert_blob(
        &db,
        &pubkey.to_string(),
        &hash,
        &bytes_vec,
        &mime_type,
        payload_size,
    )
    .await?;

    Ok(HttpResponse::Ok().json(BlobDescriptor {
        pubkey: blob.pubkey,
        hash: blob.hash,
        url: format!("{}/{}", cfg.cdn.base_url, &hash),
        r#type: blob.r#type,
        size: blob.size,
        created: blob.created,
    }))
}

async fn db_insert_blob(
    db: &SqlitePool,
    pubkey: &str,
    hash: &str,
    bytes_vec: &[u8],
    mime_type: &str,
    payload_size: i32,
) -> Result<GetBlob, sqlx::Error> {
    let now = Utc::now().timestamp();

    sqlx::query_as!(
        GetBlob,
        r#"
        INSERT INTO blobs (pubkey, hash, blob, type, size, created)
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (hash) DO NOTHING
        RETURNING *;
    "#,
        pubkey,
        hash,
        bytes_vec,
        mime_type,
        payload_size,
        now,
    )
    .fetch_one(db)
    .await
}

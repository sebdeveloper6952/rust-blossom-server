use crate::{
    api::{db_get_blob, GetBlob},
    blossom::BlobDescriptor,
    mime_type::MimeType,
};
use actix_web::{
    body::BoxBody,
    web::{Bytes, Data, ReqData},
    HttpResponse, ResponseError,
};
use chrono::Utc;
use sha256::digest;
use sqlx::SqlitePool;
use std::{collections::HashSet, convert::TryFrom};
use tracing::instrument;

use crate::config::Config;

#[derive(thiserror::Error, Debug)]
pub enum UploadError {
    #[error("failed to insert blob into DB")]
    DbInsertError(#[from] sqlx::Error),
    #[error("failed to extract payload size")]
    ExtractPayloadSizeError,
    #[error("mime type not allowed")]
    MimeTypeNotAllowed,
}

impl ResponseError for UploadError {
    fn error_response(&self) -> HttpResponse<BoxBody> {
        match self {
            UploadError::DbInsertError(_) | UploadError::ExtractPayloadSizeError => {
                HttpResponse::InternalServerError().finish()
            }
            UploadError::MimeTypeNotAllowed => HttpResponse::BadRequest()
                .json(serde_json::json!({"message": "mime type not allowed"})),
        }
    }
}

#[instrument(skip(payload, db, cfg, allowed_mime_types))]
pub async fn upload(
    pubkey: ReqData<nostr::PublicKey>,
    payload: ReqData<Bytes>,
    db: Data<SqlitePool>,
    cfg: Data<Config>,
    allowed_mime_types: Data<HashSet<MimeType>>,
) -> Result<HttpResponse, UploadError> {
    let payload_size =
        i32::try_from(payload.len()).map_err(|_| UploadError::ExtractPayloadSizeError)?;

    let bytes_vec = payload.into_inner().to_vec();
    let mime_type = match infer::get(&bytes_vec) {
        Some(t) => t.to_string(),
        _ => String::from("application/octet-stream"),
    };

    if !is_mime_type_allowed(&allowed_mime_types, &mime_type) {
        return Err(UploadError::MimeTypeNotAllowed);
    }

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

fn is_mime_type_allowed(allowed: &HashSet<MimeType>, mime_type: &str) -> bool {
    return allowed.len() == 0 || allowed.contains(&MimeType(String::from(mime_type)));
}

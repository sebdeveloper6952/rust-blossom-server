use actix_web::{
    web::{Bytes, Data, ReqData},
    HttpResponse, Responder,
};
use chrono::Utc;
use sha256::digest;
use sqlx::{sqlite::SqliteQueryResult, SqlitePool};
use std::convert::TryFrom;
use tracing::instrument;

#[instrument(skip(payload, db))]
pub async fn upload(payload: ReqData<Bytes>, db: Data<SqlitePool>) -> impl Responder {
    // TODO: handle failure
    let payload_size = i32::try_from(payload.len()).unwrap();
    let bytes_vec = payload.into_inner().to_vec();
    let mime_type = match infer::get(&bytes_vec) {
        Some(t) => t.to_string(),
        _ => String::from("application/octet-stream"),
    };
    let hash = digest(&bytes_vec);

    let _ = db_insert_blob(&db, &hash, &bytes_vec, &mime_type, payload_size).await;

    HttpResponse::Ok()
        .json(serde_json::json!({"size": payload_size, "hash": hash, "type": mime_type}))
}

async fn db_insert_blob(
    db: &SqlitePool,
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
    "#,
        "<pubkey>",
        hash,
        bytes_vec,
        mime_type,
        payload_size,
        now,
    )
    .execute(db)
    .await
}

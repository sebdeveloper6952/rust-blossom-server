use actix_web::{web, HttpResponse};
use sqlx::SqlitePool;
use tracing::instrument;

struct GetBlob {
    blob: Vec<u8>,
    r#type: String,
}

#[instrument(skip(hash, db))]
pub async fn get(
    hash: web::Path<String>,
    db: web::Data<SqlitePool>,
) -> Result<HttpResponse, actix_web::Error> {
    // TODO: error handling
    let blob = db_get_blob(&db, &hash).await.unwrap();

    Ok(HttpResponse::Ok()
        .insert_header(("Content-Type", blob.r#type))
        .body(blob.blob))
}

#[instrument(skip(path, db))]
pub async fn get_with_ext(
    path: web::Path<(String, String)>,
    db: web::Data<SqlitePool>,
) -> Result<HttpResponse, actix_web::Error> {
    // TODO: error handling
    let blob = db_get_blob(&db, &path.0).await.unwrap();

    Ok(HttpResponse::Ok()
        .insert_header(("Content-Type", blob.r#type))
        .body(blob.blob))
}

async fn db_get_blob(db: &SqlitePool, hash: &str) -> Result<GetBlob, sqlx::Error> {
    let blob = sqlx::query_as!(
        GetBlob,
        r#"
        SELECT blob, type
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

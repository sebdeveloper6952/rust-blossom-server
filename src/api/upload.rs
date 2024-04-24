use actix_web::{HttpResponse, Responder};
use tracing::instrument;

#[instrument]
pub async fn upload() -> impl Responder {
    HttpResponse::Ok().finish()
}

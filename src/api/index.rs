use actix_files::NamedFile;
use actix_web::HttpRequest;
use tracing::instrument;

#[instrument(skip(_req))]
pub async fn index_file(_req: HttpRequest) -> actix_web::Result<NamedFile> {
    Ok(NamedFile::open("index.html")?)
}

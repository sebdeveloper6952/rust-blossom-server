use actix_files::NamedFile;
use actix_web::HttpRequest;

pub async fn index_file(_req: HttpRequest) -> actix_web::Result<NamedFile> {
    Ok(NamedFile::open("index.html")?)
}

use actix_web::{
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    error::ErrorUnauthorized,
    web::Bytes,
    Error, HttpMessage,
};
use actix_web_lab::middleware::Next;

pub async fn extract_payload_size_middleware(
    mut req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    let bytes = req.extract::<Bytes>().await;

    match bytes {
        Ok(bytes) => {
            req.extensions_mut().insert(bytes);
        }
        Err(err) => return Err(ErrorUnauthorized(err)),
    }

    next.call(req).await
}

#[cfg(test)]
mod tests {
    use super::extract_payload_size_middleware;
    use actix_web::App;
    use actix_web::HttpResponse;
    use actix_web::{web, HttpMessage};
    use actix_web_lab::middleware::from_fn;
    use std::iter;

    #[actix_web::test]
    async fn test_extract_payload_size_middleware() {
        let app = actix_web::test::init_service(
            App::new().service(
                web::resource("/")
                    .wrap(from_fn(extract_payload_size_middleware))
                    .route(web::post().to(HttpResponse::Ok)),
            ),
        )
        .await;

        let dummy_size = 36194;
        let dummy_payload: Vec<u8> = iter::repeat(0).take(dummy_size).collect();
        let req = actix_web::test::TestRequest::post()
            .set_payload(dummy_payload)
            .uri("/")
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;
        let ext = resp.request().extensions();
        let bytes = ext.get::<web::Bytes>().unwrap();

        assert_eq!(bytes.len(), dummy_size);
    }
}

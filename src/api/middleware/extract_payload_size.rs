use actix_web::{
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    error::ErrorUnauthorized,
    web::Bytes,
    Error, HttpMessage,
};
use actix_web_lab::middleware::Next;

#[derive(Debug)]
pub struct PayloadSize(pub usize);

pub async fn extract_payload_size_middleware(
    mut req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    match req.extract::<Bytes>().await {
        Ok(bytes) => {
            req.extensions_mut().insert(PayloadSize(bytes.len()));
        }
        Err(err) => return Err(ErrorUnauthorized(err)),
    }

    next.call(req).await
}

#[cfg(test)]
mod tests {
    use super::extract_payload_size_middleware;
    use ::base64::prelude::*;
    use actix_web::web;
    use actix_web::App;
    use actix_web::HttpResponse;
    use actix_web_lab::middleware::from_fn;
    use nostr::prelude::*;
    use nostr_sdk::prelude::*;
    use std::time::Duration;

    #[actix_web::test]
    async fn test_auth_middleware() {
        let keys = Keys::generate();
        let auth_event = EventBuilder::new(
            Kind::Custom(24242),
            "auth event",
            vec![
                Tag::Hashtag("upload".into()),
                Tag::Size(36194),
                Tag::Expiration(Timestamp::now() + Duration::new(1000, 0)),
            ],
        )
        .to_event(&keys)
        .unwrap();

        let auth_event_json = serde_json::to_string(&auth_event).unwrap();
        let auth_event_base64 = BASE64_STANDARD.encode(auth_event_json);

        let app = actix_web::test::init_service(
            App::new().service(
                web::resource("/")
                    .wrap(from_fn(extract_payload_size_middleware))
                    .route(web::get().to(HttpResponse::Ok)),
            ),
        )
        .await;
        let req = actix_web::test::TestRequest::get()
            .uri("/")
            .insert_header(("Authorization", format!("Nostr {}", auth_event_base64)))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;

        assert!(resp.status().is_success());
    }
}

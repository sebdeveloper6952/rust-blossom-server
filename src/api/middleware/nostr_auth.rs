use crate::blossom::{is_auth_event_valid, Action};
use ::base64::prelude::*;
use actix_web::body::MessageBody;
use actix_web::error::ErrorUnauthorized;
use actix_web::web::Bytes;
use actix_web::{
    dev::{ServiceRequest, ServiceResponse},
    Error, HttpMessage,
};
use actix_web_lab::middleware::Next;
use nostr::event::Event;
use nostr_sdk::JsonUtil;

fn error_out(msg: &str) -> Error {
    return ErrorUnauthorized(serde_json::json!({"message": msg}));
}

pub async fn verify_upload(
    mut req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    let bytes = req.extract::<Bytes>().await;
    if bytes.is_err() {
        return Err(error_out("no payload found"));
    }

    let header = req.headers().get("Authorization");
    if header.is_none() {
        return Err(error_out("missing Authorization header"));
    }

    let header_value = header.unwrap().to_str();
    if header_value.is_err() {
        return Err(error_out("invalid Authorization header"));
    }

    let base64_decoded_event = BASE64_STANDARD.decode(&header_value.unwrap()[6..]);
    if base64_decoded_event.is_err() {
        return Err(error_out("invalid Auth event: failed base64 decoding"));
    }

    let event_result = Event::from_json(base64_decoded_event.unwrap());
    if event_result.is_err() {
        return Err(error_out("invalid Auth event: failed json decoding"));
    }
    let event = event_result.unwrap();

    match is_auth_event_valid(&event, Action::Upload, bytes.unwrap().len()) {
        Ok(_) => {}
        Err(e) => return Err(error_out(&e)),
    }

    req.extensions_mut().insert(event.pubkey);

    next.call(req).await
}

#[cfg(test)]
mod tests {
    use super::AuthMiddlewareFactory;
    use crate::api::extract_payload_size_middleware;
    use crate::blossom::action::Action;
    use ::base64::prelude::*;
    use actix_web::App;
    use actix_web::HttpResponse;
    use actix_web::{web, HttpMessage};
    use actix_web_lab::middleware::from_fn;
    use nostr::prelude::*;
    use nostr_sdk::prelude::*;
    use std::iter;
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
                    .wrap(AuthMiddlewareFactory::new(Action::Upload))
                    .wrap(from_fn(extract_payload_size_middleware))
                    .route(web::post().to(HttpResponse::Ok)),
            ),
        )
        .await;

        let dummy_size = 36194;
        let dummy_payload: Vec<u8> = iter::repeat(0).take(dummy_size).collect();

        let req = actix_web::test::TestRequest::post()
            .uri("/")
            .insert_header(("Authorization", format!("Nostr {}", auth_event_base64)))
            .set_payload(dummy_payload)
            .to_request();

        let resp = actix_web::test::call_service(&app, req).await;

        assert_eq!(resp.status(), 200);
    }
}

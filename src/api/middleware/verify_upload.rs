use crate::blossom::{is_auth_event_valid, Action};
use crate::config::Config;
use ::base64::prelude::*;
use actix_web::body::MessageBody;
use actix_web::error::ErrorUnauthorized;
use actix_web::web::Bytes;
use actix_web::{
    dev::{ServiceRequest, ServiceResponse},
    web, Error, HttpMessage,
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
        let cfg = req.app_data::<web::Data<Config>>();
        let error_msg = match cfg {
            Some(cfg) => format!(
                "no payload found or it doesn't fit size range: min_bytes: {}, max_bytes: {}",
                cfg.cdn.min_upload_size_bytes, cfg.cdn.max_upload_size_bytes,
            ),
            None => String::from("no payload found"),
        };

        return Err(error_out(&error_msg));
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

    let bytes = bytes.unwrap();
    match is_auth_event_valid(&event, Action::Upload, bytes.len()) {
        Ok(_) => {}
        Err(e) => return Err(error_out(&e)),
    }

    req.extensions_mut().insert(event.pubkey);
    req.extensions_mut().insert(bytes);

    next.call(req).await
}

#[cfg(test)]
mod tests {
    use super::verify_upload;
    use ::base64::prelude::*;
    use actix_web::web;
    use actix_web::App;
    use actix_web::HttpResponse;
    use actix_web_lab::middleware::from_fn;
    use nostr::prelude::*;
    use nostr_sdk::prelude::*;
    use std::iter;
    use std::time::Duration;

    #[actix_web::test]
    async fn test_verify_upload_middleware() {
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
                    .wrap(from_fn(verify_upload))
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

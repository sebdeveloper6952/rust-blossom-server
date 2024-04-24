use crate::blossom::action::Action;
use crate::blossom::auth::is_auth_event_valid;
use ::base64::prelude::*;
use actix_web::{
    body::EitherBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpResponse,
};
use futures_util::future::{FutureExt, LocalBoxFuture};
use nostr::event::Event;
use nostr_sdk::JsonUtil;
use std::future::{ready, Ready};

pub struct VerifyBodySizeMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for VerifyBodySizeMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let fut = self.service.call(req);
        Box::pin(async move {
            let res = fut.await?;
            Ok(res.map_into_left_body())
        })
    }
}

pub struct VerifyBodySizeMiddlewareFactory {}

impl<S, B> Transform<S, ServiceRequest> for VerifyBodySizeMiddlewareFactory
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = VerifyBodySizeMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(VerifyBodySizeMiddleware { service }))
    }
}

#[cfg(test)]
mod tests {
    use super::VerifyBodySizeMiddlewareFactory;
    use crate::blossom::action::Action;
    use ::base64::prelude::*;
    use actix_web::web;
    use actix_web::App;
    use actix_web::HttpResponse;
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
                    .wrap(VerifyBodySizeMiddlewareFactory {})
                    .route(web::get().to(HttpResponse::Ok)),
            ),
        )
        .await;
        let req = actix_web::test::TestRequest::get()
            .uri("/")
            .insert_header(("Authorization", format!("Nostr {}", auth_event_base64)))
            .to_request();
        let resp = actix_web::test::call_service(&app, req).await;

        println!("status: {}", resp.status());
        assert!(resp.status().is_success());
    }
}

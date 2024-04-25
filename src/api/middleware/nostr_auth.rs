use crate::blossom::action::Action;
use crate::blossom::auth::is_auth_event_valid;
use ::base64::prelude::*;
use actix_web::web::Bytes;
use actix_web::{
    body::EitherBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage, HttpResponse,
};
use futures_util::future::{FutureExt, LocalBoxFuture};
use nostr::event::Event;
use nostr_sdk::JsonUtil;
use std::future::{ready, Ready};

pub struct AuthMiddleware<S> {
    action: Action,
    service: S,
}

impl<S, B> Service<ServiceRequest> for AuthMiddleware<S>
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
        let srv_req = ServiceRequest::from_request(req.request().clone());
        let req_ext = srv_req.extensions();
        let payload_size = req_ext.get::<Bytes>();

        if payload_size.is_none() {
            let http_res = HttpResponse::InternalServerError().finish();
            let res = ServiceResponse::new(req.request().clone(), http_res);
            return (async move { Ok(res.map_into_right_body()) }).boxed_local();
        }

        let error_out = |msg: &str| -> Self::Future {
            let http_res = HttpResponse::Unauthorized().json(serde_json::json!({"message": msg}));
            let res = ServiceResponse::new(req.request().clone(), http_res);
            (async move { Ok(res.map_into_right_body()) }).boxed_local()
        };

        let header = req.headers().get("Authorization");
        if header.is_none() {
            return error_out("missing Auth event");
        }

        let header_value = header.unwrap().to_str();
        if header_value.is_err() {
            return error_out("invalid Auth event");
        }

        let base64_decoded_event = BASE64_STANDARD.decode(&header_value.unwrap()[6..]);
        if base64_decoded_event.is_err() {
            return error_out("invalid Auth event: failed base64 decoding");
        }

        let event = Event::from_json(base64_decoded_event.unwrap());
        if event.is_err() {
            return error_out("invalid Auth event: failed json decoding");
        }

        match is_auth_event_valid(
            &event.unwrap(),
            self.action.clone(),
            payload_size.unwrap().len(),
        ) {
            Ok(_) => {}
            Err(e) => return error_out(&e),
        }

        let fut = self.service.call(req);
        Box::pin(async move {
            let res = fut.await?;
            Ok(res.map_into_left_body())
        })
    }
}

pub struct AuthMiddlewareFactory {
    action: Action,
}

impl AuthMiddlewareFactory {
    pub fn new(action: Action) -> Self {
        Self { action }
    }
}

impl<S, B> Transform<S, ServiceRequest> for AuthMiddlewareFactory
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = AuthMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthMiddleware {
            action: self.action.clone(),
            service,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::AuthMiddlewareFactory;
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
                    .wrap(AuthMiddlewareFactory::new(Action::Upload))
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

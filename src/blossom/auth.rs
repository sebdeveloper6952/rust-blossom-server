use crate::blossom::action::Action;
use ::base64::prelude::*;
use actix_web::{
    body::EitherBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpResponse,
};
use futures_util::future::{FutureExt, LocalBoxFuture};
use nostr::event::Event;
use nostr::{Alphabet, Kind, SingleLetterTag, TagKind, Timestamp};
use nostr_sdk::JsonUtil;
use std::future::{ready, Ready};
use std::str::FromStr;

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
        let error_out = || -> Self::Future {
            let http_res = HttpResponse::Unauthorized().finish();
            let res = ServiceResponse::new(req.request().clone(), http_res);
            return (async move { Ok(res.map_into_right_body()) }).boxed_local();
        };

        let header = req.headers().get("Authorization");
        if header.is_none() {
            return error_out();
        }

        let header_value = header.unwrap().to_str();
        if header_value.is_err() {
            return error_out();
        }

        let base64_decoded_event = BASE64_STANDARD.decode(&header_value.unwrap()[6..]);
        if base64_decoded_event.is_err() {
            return error_out();
        }

        let event = Event::from_json(base64_decoded_event.unwrap());
        if event.is_err() {
            return error_out();
        }

        let valid = is_auth_event_valid(&event.unwrap(), self.action.clone(), 36194);
        if valid.is_err() {
            return error_out();
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

/// logic to actually validate if an event is a valid blossom authentication event
fn is_auth_event_valid(event: &Event, action: Action, size: usize) -> Result<(), String> {
    // TODO: validate event signature

    if event.kind() != Kind::Custom(24242) {
        return Err("kind must be 24242".into());
    }

    if event.created_at() > Timestamp::now() {
        return Err("created_at must be in the past".into());
    }

    match event.tags.iter().find(|t| {
        t.kind()
            == TagKind::SingleLetter(SingleLetterTag {
                character: Alphabet::T,
                uppercase: false,
            })
    }) {
        Some(tag) => {
            if let Some(tag_value) = tag.content() {
                match Action::from_str(&tag_value.to_string()) {
                    Ok(tag_action) => {
                        if tag_action != action {
                            return Err("action doesn't match".into());
                        }
                    }
                    _ => return Err("invalid action".into()),
                }
            }
        }
        _ => {
            return Err("t tag must be set".into());
        }
    }

    match event.tags.iter().find(|t| t.kind() == TagKind::Expiration) {
        Some(tag) => {
            if let Some(tag_value) = tag.content() {
                match Timestamp::from_str(&tag_value.to_string()) {
                    Ok(exp) => {
                        if exp < Timestamp::now() {
                            return Err("expiration must be in the future".into());
                        }
                    }
                    _ => return Err("invalid expiration".into()),
                }
            }
        }
        _ => {
            return Err("expiration tag must be set".into());
        }
    }

    if action == Action::Upload {
        match event.tags.iter().find(|t| t.kind() == TagKind::Size) {
            Some(tag) => {
                if let Some(tag_value) = tag.content() {
                    match tag_value.to_string().parse::<usize>() {
                        Ok(tag_size) => {
                            if tag_size != size {
                                return Err("size doesn't match".into());
                            }
                        }
                        _ => return Err("invalid size".into()),
                    }
                }
            }
            _ => {
                return Err("size tag must be set".into());
            }
        }
    }

    if action == Action::Delete {
        match event.tags.iter().find(|t| {
            t.kind()
                == TagKind::SingleLetter(SingleLetterTag {
                    character: Alphabet::X,
                    uppercase: false,
                })
        }) {
            None => {
                return Err("x tag must be set".into());
            }
            _ => {}
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::is_auth_event_valid;
    use super::AuthMiddlewareFactory;
    use crate::blossom::action::Action;
    use ::base64::prelude::*;
    use actix_web::web;
    use actix_web::App;
    use actix_web::HttpResponse;
    use nostr::prelude::*;
    use nostr_sdk::prelude::*;
    use std::time::Duration;

    #[test]
    fn valid_event_passes_through() {
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

        let result = is_auth_event_valid(&auth_event, Action::Upload, 36194);

        assert!(result.is_ok());
    }

    #[test]
    fn different_kind_fails() {
        let keys = Keys::generate();
        let auth_event = EventBuilder::new(
            Kind::Custom(69420),
            "auth event",
            vec![
                Tag::Hashtag("get".into()),
                Tag::Size(36194),
                Tag::Expiration(Timestamp::now() + Duration::new(1000, 0)),
            ],
        )
        .to_event(&keys)
        .unwrap();

        let result = is_auth_event_valid(&auth_event, Action::Upload, 36194);

        assert!(result.is_err());
    }
    #[test]
    fn different_action_fails() {
        let keys = Keys::generate();
        let auth_event = EventBuilder::new(
            Kind::Custom(24242),
            "auth event",
            vec![
                Tag::Hashtag("get".into()),
                Tag::Size(36194),
                Tag::Expiration(Timestamp::now() + Duration::new(1000, 0)),
            ],
        )
        .to_event(&keys)
        .unwrap();

        let result = is_auth_event_valid(&auth_event, Action::Upload, 36194);

        assert!(result.is_err());
    }

    #[test]
    fn different_size_fails() {
        let keys = Keys::generate();
        let auth_event = EventBuilder::new(
            Kind::Custom(24242),
            "auth event",
            vec![
                Tag::Hashtag("upload".into()),
                Tag::Size(36193),
                Tag::Expiration(Timestamp::now() + Duration::new(1000, 0)),
            ],
        )
        .to_event(&keys)
        .unwrap();

        let result = is_auth_event_valid(&auth_event, Action::Upload, 36194);

        assert!(result.is_err());
    }

    #[test]
    fn expiration_in_the_past_fails() {
        let keys = Keys::generate();
        let auth_event = EventBuilder::new(
            Kind::Custom(24242),
            "auth event",
            vec![
                Tag::Hashtag("upload".into()),
                Tag::Size(36194),
                Tag::Expiration(Timestamp::now() - Duration::new(1000, 0)),
            ],
        )
        .to_event(&keys)
        .unwrap();

        let result = is_auth_event_valid(&auth_event, Action::Upload, 36194);

        assert!(result.is_err());
    }

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

        println!("status: {}", resp.status());
        assert!(resp.status().is_success());
    }
}

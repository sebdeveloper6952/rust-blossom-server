use crate::blossom::Action;
use actix_web::{
    body::EitherBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    error::ErrorInternalServerError,
    web::Data,
    Error, HttpMessage, HttpResponse,
};
use futures_util::{future::LocalBoxFuture, FutureExt};
use nostr_sdk::PublicKey;
use std::{
    collections::HashSet,
    future::{ready, Ready},
};

pub struct PubkeyWhitelistMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for PubkeyWhitelistMiddleware<S>
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
        let srv = ServiceRequest::from_request(req.request().clone());
        let ext = srv.extensions();
        let authed_pubkey = ext.get::<PublicKey>();
        if authed_pubkey.is_none() {
            return (async move { Err(ErrorInternalServerError("failed to find a pubkey")) })
                .boxed_local();
        }

        let whitelisted_pks = req.app_data::<Data<HashSet<&str>>>();
        if whitelisted_pks.is_some() {
            let pks = whitelisted_pks.unwrap();
            let pk = authed_pubkey.unwrap().to_string();
            if pks.len() > 0 && !pks.contains(pk.as_str()) {
                let http_res = HttpResponse::Forbidden().finish();
                let res = ServiceResponse::new(req.request().clone(), http_res);
                return (async move { Ok(res.map_into_right_body()) }).boxed_local();
            }
        }

        let fut = self.service.call(req);
        Box::pin(async move {
            let res = fut.await?;
            Ok(res.map_into_left_body())
        })
    }
}

pub struct PubkeyWhitelistMiddlewareFactory {}

impl<S, B> Transform<S, ServiceRequest> for PubkeyWhitelistMiddlewareFactory
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = PubkeyWhitelistMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(PubkeyWhitelistMiddleware { service }))
    }
}

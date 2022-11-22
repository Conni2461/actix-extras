use std::{future::Future, pin::Pin, rc::Rc};

use actix_utils::future::{ok, Ready};
use actix_web::{
    body::EitherBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    http::StatusCode,
    Error, HttpResponse,
};

use crate::{DataSource, Error as LimitationError, Limiter};

/// Rate limit middleware.
#[derive(Debug)]
#[non_exhaustive]
pub struct RateLimiter<T: DataSource> {
    limiter: Rc<Limiter<T>>,
}

impl<T: DataSource> RateLimiter<T> {
    pub fn new(source: Limiter<T>) -> Self {
        Self {
            limiter: Rc::new(source),
        }
    }
}

impl<S, B, T> Transform<S, ServiceRequest> for RateLimiter<T>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
    T: DataSource + 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Transform = RateLimiterMiddleware<S, T>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(RateLimiterMiddleware {
            service: Rc::new(service),
            limiter: Rc::clone(&self.limiter),
        })
    }
}

/// Rate limit middleware service.
#[derive(Debug)]
pub struct RateLimiterMiddleware<S, T: DataSource> {
    service: Rc<S>,
    limiter: Rc<Limiter<T>>,
}

impl<S, B, T> Service<ServiceRequest> for RateLimiterMiddleware<S, T>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
    T: DataSource + 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // A misconfiguration of the Actix App will result in a **runtime** failure, so the expect
        // method description is important context for the developer.
        let limiter = Rc::clone(&self.limiter);

        let key = (limiter.get_key_fn)(&req);
        let service = Rc::clone(&self.service);

        let key = match key {
            Some(key) => key,
            None => {
                return Box::pin(async move {
                    service
                        .call(req)
                        .await
                        .map(ServiceResponse::map_into_left_body)
                });
            }
        };

        Box::pin(async move {
            let status = limiter.count(key.to_string()).await;

            if let Err(err) = status {
                match err {
                    LimitationError::LimitExceeded(_) => {
                        log::warn!("Rate limit exceed error for {}", key);

                        Ok(req.into_response(
                            HttpResponse::new(StatusCode::TOO_MANY_REQUESTS).map_into_right_body(),
                        ))
                    }
                    LimitationError::Track(s) => {
                        log::error!("Failed to track client: {}", s);

                        Ok(req.into_response(
                            HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR)
                                .map_into_right_body(),
                        ))
                    }
                    _ => {
                        log::error!("Count failed: {}", err);

                        Ok(req.into_response(
                            HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR)
                                .map_into_right_body(),
                        ))
                    }
                }
            } else {
                service
                    .call(req)
                    .await
                    .map(ServiceResponse::map_into_left_body)
            }
        })
    }
}

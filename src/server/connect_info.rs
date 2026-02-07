//! Optional ConnectInfo extractor for handlers that can run without it in tests.

use axum::extract::{ConnectInfo, FromRequestParts};
use axum::http::request::Parts;
use std::convert::Infallible;
use std::net::SocketAddr;

/// ConnectInfo wrapper that yields `None` when connect info isn't available.
#[derive(Debug, Clone, Copy)]
pub struct MaybeConnectInfo(pub Option<SocketAddr>);

impl<S> FromRequestParts<S> for MaybeConnectInfo
where
    S: Send + Sync,
{
    type Rejection = Infallible;

    fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> + Send {
        let addr = parts
            .extensions
            .get::<ConnectInfo<SocketAddr>>()
            .map(|info| info.0);
        async move { Ok(MaybeConnectInfo(addr)) }
    }
}

use axum::{routing::get, Router};
use common::{
    axum::{
        self,
        routing::{on, MethodFilter},
        Extension,
    },
    tracing,
};

use context::context::Context;
use pod_exec::{container_list, handler};

pub async fn init_router() -> Router {
    let ctx = Context::new()
        .await
        .map_err(|err| {
            tracing::error!("Get context err, {}", err);
        })
        .unwrap();

    Router::new()
        .route("/health", get(|| async { "Hello, World!" }))
        .route("/container", on(MethodFilter::GET, container_list))
        .route(
            "/namespace/:namespace/pod/:pod/container/:container",
            on(MethodFilter::GET, handler),
        )
        .layer(Extension(ctx))
}

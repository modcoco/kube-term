pub mod connector;
pub mod model;
pub mod msg_handle;
pub mod services;

use axum::{extract::WebSocketUpgrade, response::Response};
use common::{
    axum::{
        self,
        extract::{Query, RawPathParams},
        response::IntoResponse,
        Extension,
    },
    tracing,
};
use connector::ContainerCoords;
use context::context::Context;
use model::ContainerQuery;
use services::{get_container_list, get_ns_list, handle_socket};
use std::borrow::Cow;
use util::{err::AxumErr, rsp::Rsp};

pub async fn handler(ws: WebSocketUpgrade, raw_path_params: RawPathParams) -> Response {
    let coords = ContainerCoords::default().populate_from_raw_path_params(&raw_path_params);
    tracing::info!("{:?}", coords);

    let protocols: Vec<Cow<'static, str>> = vec![Cow::Borrowed("echo-protocol")];
    ws.protocols(protocols)
        .on_upgrade(|axum_socket| handle_socket(axum_socket, coords))
}

pub async fn container_list(
    Query(req): Query<ContainerQuery>,
    Extension(ctx): Extension<Context>,
) -> Result<impl IntoResponse, AxumErr> {
    tracing::info!("Get container list");
    let container_res = get_container_list(req, ctx).await?;

    Ok(Rsp::success_with_optional_biz_status(
        container_res,
        "Data fetched successfully.",
        Some(1),
    ))
}

pub async fn ns_list(Extension(ctx): Extension<Context>) -> Result<impl IntoResponse, AxumErr> {
    tracing::info!("Get namespace list");
    let namespace_list = get_ns_list(ctx).await?;

    Ok(Rsp::success_with_data(
        namespace_list,
        "Data fetched successfully.",
    ))
}

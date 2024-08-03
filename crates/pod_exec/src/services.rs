use axum::extract::ws::WebSocket;
use common::{
    anyhow,
    axum::{self, extract::ws::Message},
    tokio::{self, sync::mpsc},
    tracing,
};
use context::context::Context;
use kube::{
    k8s_openapi::api::core::v1::{Namespace, Pod},
    kube_runtime::{api::ListParams, Api},
    ServiceAccountToken,
};

use crate::{
    connector::{self, ContainerCoordsOptional},
    model::{ContainerQuery, ContainerRsp, ContainerSimpleInfo, NamespaceSimpleInfo},
    msg_handle,
};

use connector::{pod_exec_connector, ContainerCoords, PodExecParams, PodExecUrl};
use msg_handle::handle_websocket;

pub async fn get_ns_list(ctx: Context) -> Result<Vec<NamespaceSimpleInfo>, anyhow::Error> {
    let namespaces: Api<Namespace> = Api::all(ctx.kube_client.clone());
    let lp = ListParams::default();
    let ns_list = namespaces.list(&lp).await?;

    let mut namespace_list = Vec::new();
    for ns in ns_list.items {
        let ns_name = ns.metadata.name.as_deref().unwrap_or("<unknown>");
        let ns_uid = ns.metadata.uid.as_deref().unwrap_or("<unknown>");
        let resource_version = ns
            .metadata
            .resource_version
            .as_deref()
            .unwrap_or("<unknown>");

        let ns = NamespaceSimpleInfo {
            id: ns_uid.to_string(),
            name: ns_name.to_string(),
            resource_version: resource_version.to_string(),
            r#type: "Kube".to_string(),
        };
        namespace_list.push(ns);
    }
    Ok(namespace_list)
}

pub async fn get_container_list(
    req: ContainerQuery,
    ctx: Context,
) -> Result<ContainerRsp, anyhow::Error> {
    let pods: Api<Pod> = Api::namespaced(
        ctx.kube_client.clone(),
        &req.ns.unwrap_or("default".to_owned()),
    );

    let mut lp = ListParams::default().limit(req.page_size.unwrap_or(4).try_into().unwrap());
    if let Some(token) = req.page_token {
        lp = lp.continue_token(&token);
    }

    let pods = pods.list(&lp).await?;

    let continue_token = &pods.metadata.continue_;
    tracing::info!("continue_koken {:?}", continue_token);

    let mut container_list = Vec::new();
    for p in &pods {
        let namespace = p.metadata.namespace.clone().unwrap_or_default();
        let pod_name = p.metadata.name.clone().unwrap_or_default();
        let pod_status = p.status.clone().unwrap_or_default();
        let pod_ip = pod_status.pod_ip.unwrap_or("<unkonwn>".to_owned());
        let pod_phase = pod_status.phase.unwrap_or("<unkonwn>".to_owned());

        if let Some(spec) = &p.spec {
            for container in &spec.containers {
                let container_name = container.name.clone();
                let container_image = container.image.clone().unwrap_or("<unkonwn>".to_owned());

                tracing::info!(
                    "namespace: {:?}, podname: {:?}, container_name: {:?}",
                    namespace,
                    pod_name,
                    container_name
                );

                let container_coords = ContainerCoordsOptional {
                    namespace: Some(namespace.clone()),
                    pod: Some(pod_name.clone()),
                    container: Some(container_name),
                };
                let container = ContainerSimpleInfo {
                    container: container_coords,
                    pod_ip: pod_ip.clone(),
                    pod_phase: pod_phase.clone(),
                    container_image,
                };
                container_list.push(container)
            }
        }
    }

    let container_res = ContainerRsp {
        container_list,
        page_token: continue_token.clone(),
    };

    Ok(container_res)
}

pub async fn handle_socket(mut axum_socket: WebSocket, coords: ContainerCoords) {
    let sat = ServiceAccountToken::new();

    let pod_exec_url = PodExecUrl::default().get_exec_url(&sat.kube_host, &sat.kube_port, &coords);
    let pod_exec_params = PodExecParams::default().get_pod_exec_params(&coords);

    let (tx_web, mut rx_web) = mpsc::channel::<Message>(100);
    let (tx_kube, mut rx_kube) = mpsc::channel(100);

    let conn = pod_exec_connector(&sat, &pod_exec_url, &pod_exec_params).await;
    match conn {
        Ok(mut kube_ws_stream) => {
            let mut closed = false;
            tokio::spawn(async move {
                handle_websocket(
                    &mut kube_ws_stream,
                    &mut rx_web,
                    &tx_kube,
                    &mut closed,
                    None,
                )
                .await;
            });
        }
        Err(err) => {
            tracing::error!("ERROR, {}", err)
        }
    };

    loop {
        tokio::select! {
            Some(client_msg) = axum_socket.recv() => {
                let client_msg = if let Ok(client_msg) = client_msg {
                    tracing::debug!("Received from client: {:?}", client_msg);
                    client_msg
                } else {
                    tracing::info!("Client disconnected, the msg isn't ok");
                    return;
                };

                if tx_web.send(client_msg).await.is_err() {
                    tracing::info!("Failed to send message to channel");
                }
            },
            Some(kube_msg) = rx_kube.recv() => {
                tracing::debug!("Received from kubernetes: {}", kube_msg);
                let kube_msg = Message::Text(kube_msg);
                if axum_socket.send(kube_msg).await.is_err() {
                    tracing::info!("Client disconnected, failed to send message");
                }
            }
        }
    }
}

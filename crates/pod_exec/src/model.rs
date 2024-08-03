use crate::connector::ContainerCoordsOptional;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContainerQuery {
    pub ns: Option<String>,
    pub page_size: Option<i8>,
    pub page_token: Option<String>,
}

#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContainerSimpleInfo {
    #[serde(flatten)]
    pub container: ContainerCoordsOptional,
    pub pod_ip: String,
    pub pod_phase: String,
    pub container_image: String,
}

#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContainerRsp {
    pub container_list: Vec<ContainerSimpleInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_token: Option<String>,
}

#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NamespaceSimpleInfo {
    pub id: String,
    pub name: String,
    pub resource_version: String,
    pub r#type: String,
}

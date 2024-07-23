#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use common::axum::http::HeaderMap;
    use common::reqwest::blocking::Client;
    use common::reqwest::header::AUTHORIZATION;
    use common::reqwest::Certificate;
    use common::{anyhow, serde_json, toml};
    use common::{tokio, tracing};
    use k8s_openapi::api::core::v1::Pod;
    use k8s_openapi::api::core::v1::{ConfigMap, Namespace};
    use kube::ServiceAccountToken;
    use kube_runtime::{api::ListParams, Api, Client as KubeClient, Config};
    use util::url_https_builder;

    #[test]
    fn str_trimmed() {
        let str = "nvidia.com";
        let trimmed_str = str.trim_end_matches(".com");
        println!("{}", trimmed_str);
    }

    #[test]
    fn test_env() {
        let ps = ServiceAccountToken::new();
        println!("{:?}", ps)
    }

    #[test]
    fn rquest_tls() -> Result<(), anyhow::Error> {
        let (_handle, _guard) = logger::logger_trace::init_logger("test_kube", false);

        let sat = ServiceAccountToken::new();
        let kubernetes_token = sat.token;
        let kubernetes_cert = Certificate::from_pem(&sat.cacrt)?;

        let client = Client::builder()
            .use_rustls_tls()
            .add_root_certificate(kubernetes_cert)
            .build()?;

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            format!("Bearer {}", kubernetes_token).parse()?,
        );

        let url = url_https_builder(&sat.kube_host, &sat.kube_port, Some("/version"));
        let response = client.get(url).headers(headers).send()?;

        tracing::info!("{}", response.status());
        tracing::info!("{}", response.text()?);
        Ok(())
    }

    #[test]
    fn rquest_pods() -> Result<(), anyhow::Error> {
        let (_handle, _guard) = logger::logger_trace::init_logger("test_boot", false);

        let sat = ServiceAccountToken::new();
        let kubernetes_token = sat.token;
        let kubernetes_cert = Certificate::from_pem(&sat.cacrt)?;

        let client = Client::builder()
            .use_rustls_tls()
            .add_root_certificate(kubernetes_cert)
            .build()?;

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            format!("Bearer {}", kubernetes_token).parse()?,
        );

        let url = url_https_builder(
            &sat.kube_host,
            &sat.kube_port,
            Some("/api/v1/namespaces/default/pods"),
        );
        let response = client.get(url).headers(headers).send()?;

        tracing::info!("{}", response.status());
        tracing::info!("{}", response.text()?);
        Ok(())
    }

    #[tokio::test]
    pub async fn rquest_pods_kube() -> Result<(), anyhow::Error> {
        rustls::crypto::ring::default_provider()
            .install_default()
            .expect("Failed to install rustls crypto provider");

        let config = Config::infer().await?;
        // let config = Config::incluster()?;
        let client = KubeClient::try_from(config)?;

        let pods: Api<Pod> = Api::namespaced(client.clone(), "default");

        let lp = ListParams::default().limit(10);
        let pods = pods.list(&lp).await?;

        for p in pods {
            // println!("name {:?}", p);
            let json_string = serde_json::to_string(&p).expect("Failed to convert to JSON");
            println!("name {}", json_string);
        }

        let namespaces: Api<Namespace> = Api::all(client);

        let lp = ListParams::default().limit(2);
        let ns_list = namespaces.list(&lp).await?;

        for ns in ns_list.items {
            let ns_name = ns.metadata.name.as_deref().unwrap_or("<unknown>");
            println!("Namespace name: {}", ns_name);
        }
        let continue_token = ns_list.metadata.continue_.clone().unwrap_or_default();

        let lp = ListParams::default()
            .limit(2)
            .continue_token(&continue_token);
        let ns_list = namespaces.list(&lp).await?;

        for ns in ns_list.items {
            let ns_name = ns.metadata.name.as_deref().unwrap_or("<unknown>");
            println!("Namespace2 name: {}", ns_name);
        }

        Ok(())
    }

    #[tokio::test]
    pub async fn get_configmap() -> Result<(), anyhow::Error> {
        rustls::crypto::ring::default_provider()
            .install_default()
            .expect("Failed to install rustls crypto provider");

        let config = Config::infer().await?;
        // let config = Config::incluster()?;
        let client = KubeClient::try_from(config)?;

        let namespace = "default";
        let config_map_name = "webterm-cm";

        let config_maps: Api<ConfigMap> = Api::namespaced(client, namespace);

        match config_maps.get(config_map_name).await {
            Ok(config_map) => {
                let pretty_config_map =
                    serde_json::to_string_pretty(&config_map).unwrap_or_default();
                println!("Config Map: {}", pretty_config_map);
            }
            Err(e) => {
                eprintln!("Error fetching config map: {:?}", e);
            }
        }

        Ok(())
    }

    #[tokio::test]
    pub async fn get_all_configmap_value() -> Result<(), anyhow::Error> {
        rustls::crypto::ring::default_provider()
            .install_default()
            .expect("Failed to install rustls crypto provider");

        let config = Config::infer().await?;
        // let config = Config::incluster()?;
        let client = KubeClient::try_from(config)?;

        // 定义命名空间
        let namespace = "default";

        // 创建一个 Api<ConfigMap> 实例
        let config_maps: Api<ConfigMap> = Api::namespaced(client, namespace);

        // 列出所有的 ConfigMap
        let lp = ListParams::default();
        match config_maps.list(&lp).await {
            Ok(config_map_list) => {
                for config_map in config_map_list {
                    if let Some(name) = &config_map.metadata.name {
                        println!("ConfigMap name: {}", name);
                        if let Some(data) = &config_map.data {
                            println!("ConfigMap data: {:?}", data);
                        } else {
                            println!("ConfigMap does not contain data.");
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Error listing config maps: {:?}", e);
            }
        }

        Ok(())
    }

    #[tokio::test]
    pub async fn get_configmap_value() -> Result<(), anyhow::Error> {
        rustls::crypto::ring::default_provider()
            .install_default()
            .expect("Failed to install rustls crypto provider");

        let config = Config::infer().await?;
        // let config = Config::incluster()?;
        let client = KubeClient::try_from(config)?;

        // 定义命名空间和 ConfigMap 名称
        let namespace = "default";
        let config_map_name = "webterm-cm";

        // 创建一个 Api<ConfigMap> 实例
        let config_maps: Api<ConfigMap> = Api::namespaced(client, namespace);

        // 获取指定的 ConfigMap
        match config_maps.get(config_map_name).await {
            Ok(config_map) => {
                println!("ConfigMap name: {}", config_map_name);
                if let Some(data) = config_map.data {
                    println!("ConfigMap data: {:?}", data);
                    println!("ConfigMap data: {:?}", data.get("config.yaml"));
                } else {
                    println!("ConfigMap does not contain data.");
                }
            }
            Err(e) => {
                eprintln!("Error fetching ConfigMap: {:?}", e);
            }
        }

        Ok(())
    }

    #[tokio::test]
    pub async fn get_all_configmap_name() -> Result<(), anyhow::Error> {
        rustls::crypto::ring::default_provider()
            .install_default()
            .expect("Failed to install rustls crypto provider");

        let config = Config::infer().await?;
        // let config = Config::incluster()?;
        let client = KubeClient::try_from(config)?;

        let namespace = "default";

        // 创建一个 Api<ConfigMap> 实例
        let config_maps: Api<ConfigMap> = Api::namespaced(client, namespace);

        // 列出所有的 ConfigMap
        let lp = ListParams::default();
        match config_maps.list(&lp).await {
            Ok(config_map_list) => {
                for config_map in config_map_list {
                    if let Some(name) = config_map.metadata.name {
                        println!("ConfigMap name: {}", name);
                    }
                }
            }
            Err(e) => {
                eprintln!("Error listing config maps: {:?}", e);
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn get_cm_toml_value() -> Result<(), Box<dyn std::error::Error>> {
        rustls::crypto::ring::default_provider()
            .install_default()
            .expect("Failed to install rustls crypto provider");

        let config = Config::infer().await?;
        // let config = Config::incluster()?;
        let client = KubeClient::try_from(config)?;

        let namespace = "default";
        let config_map_name = "test";

        let config_maps: Api<ConfigMap> = Api::namespaced(client, namespace);

        match config_maps.get(config_map_name).await {
            Ok(config_map) => {
                println!("ConfigMap name: {}", config_map_name);
                if let Some(data) = config_map.data {
                    if let Some(config_toml) = data.get("config.toml") {
                        let toml_data: HashMap<String, String> = toml::from_str(config_toml)?;
                        if let Some(develop_image_tag) = toml_data.get("TomlKey") {
                            println!("TomlKey: {}", develop_image_tag);
                        } else {
                            println!("Key 'TomlKey' not found in config.toml.");
                        }
                    } else {
                        println!("Key 'config.toml' not found in ConfigMap.");
                    }
                } else {
                    println!("ConfigMap does not contain data.");
                }
            }
            Err(e) => {
                eprintln!("Error fetching ConfigMap: {:?}", e);
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn get_cm_toml_value2() -> Result<(), anyhow::Error> {
        rustls::crypto::ring::default_provider()
            .install_default()
            .expect("Failed to install rustls crypto provider");

        let config = Config::infer().await?;
        // let config = Config::incluster()?;
        let client = KubeClient::try_from(config)?;
        let namespace = "test";
        let config_map_name = "config";
        let config_maps: Api<ConfigMap> = Api::namespaced(client, namespace);

        let config_map = config_maps.get(config_map_name).await?;

        let config_toml = config_map
            .data
            .as_ref()
            .and_then(|data| data.get("config.toml"))
            .ok_or_else(|| anyhow::anyhow!("ConfigMap does not contain 'config.toml'"))?;

        let toml_data: HashMap<String, toml::Value> = toml::from_str(config_toml)?;
        let develop_image_tag = toml_data
            .get("test")
            .ok_or_else(|| anyhow::anyhow!("Key 'test' not found in config.toml"))?;

        tracing::info!("test: {}", develop_image_tag);
        Ok(())
    }
}

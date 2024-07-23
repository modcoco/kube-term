#[cfg(test)]
mod tests {
    use common::axum::http::HeaderMap;
    use common::reqwest::blocking::Client;
    use common::reqwest::header::AUTHORIZATION;
    use common::reqwest::Certificate;
    use common::{anyhow, tracing};
    use common::{base64, tokio};
    use kube::ServiceAccountToken;
    use pod_exec::connector::{pod_exec_connector, PodExecParams, PodExecPath, PodExecUrl};
    use pod_exec::msg_handle::{handle_websocket, stdin_reader};
    use tokio::sync::mpsc;
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

    //
    //
    #[test]
    fn test_base64_decode() {
        let input = "4eyJXaWR0aCI6ODAsIkhlaWdodCI6MjR9";
        // let input = "0DQ==";
        let input: String = input.chars().skip(1).collect();
        let input =
            base64::Engine::decode(&base64::prelude::BASE64_STANDARD, input).unwrap_or_default();

        println!("{:?}", input)
    }

    #[test]
    fn rquest_tls() -> Result<(), anyhow::Error> {
        let _ = logger::logger_trace::init_logger("test_tls", false);

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

    #[tokio::test]
    async fn kube_cmd() {
        let _ = logger::logger_trace::init_logger("test_cmd", false);

        let sat = ServiceAccountToken::new();
        let pod_exec_url = PodExecUrl {
            domain: String::from(&sat.kube_host),
            port: String::from(&sat.kube_port),
            path: PodExecPath {
                base_path: String::from("/api/v1"),
                namespace: String::from("/namespaces/default"),
                pod: String::from("/pods/web-term-559fdfcd89-gndr5"),
                tail_path: String::from("/exec"),
            },
        };
        let pod_exec_params = PodExecParams {
            container: "web-term".to_string(),
            stdin: true,
            stdout: true,
            stderr: true,
            tty: true,
            command: "bash".to_string(),
            pretty: true,
            follow: true,
        };

        let (tx_cmd, mut rx_cmd) = mpsc::channel(100);
        let (tx_ws, mut rx_ws) = mpsc::channel(100);

        stdin_reader(tx_cmd).await;

        let conn = pod_exec_connector(&sat, &pod_exec_url, &pod_exec_params).await;
        match conn {
            Ok(mut ws_stream) => {
                let mut closed = false;
                tokio::spawn(async move {
                    handle_websocket(&mut ws_stream, &mut rx_cmd, &tx_ws, &mut closed, Some(true))
                        .await;
                });
            }
            Err(err) => {
                tracing::error!("ERROR, {}", err)
            }
        };
        while let Some(msg) = rx_ws.recv().await {
            tracing::info!("Received from kubernetes: {}", msg);
        }
    }
}

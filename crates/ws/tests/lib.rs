pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::axum::http::HeaderMap;
    use common::reqwest::blocking::Client;
    use common::reqwest::header::AUTHORIZATION;
    use common::reqwest::Certificate;
    use common::{anyhow, tracing, url_https_builder};
    use kube::ServiceAccountToken;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }

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
        logger::logger_trace::init_logger();

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
}

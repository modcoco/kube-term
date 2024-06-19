pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::blocking::Client;
    use reqwest::header::AUTHORIZATION;
    use reqwest::Certificate;
    use std::env;
    use std::fs::read;

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
    fn rquest_tls() -> Result<(), Box<dyn std::error::Error>> {
        let kubernetes_token = env::var("KUBERNETES_TOKEN")?;
        let ca_cert_path = env::var("CA_CERT_PATH")?;
        let kubernetes_service_host = env::var("KUBERNETES_SERVICE_HOST")?;
        let kubernetes_service_port = env::var("KUBERNETES_SERVICE_PORT")?;

        let cert = read(ca_cert_path)?;
        let cert = Certificate::from_pem(&cert)?;

        let client = Client::builder()
            .use_rustls_tls()
            .add_root_certificate(cert)
            .build()?;

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            format!("Bearer {}", kubernetes_token).parse()?,
        );

        let url = format!(
            "https://{}:{}/version",
            kubernetes_service_host, kubernetes_service_port
        );

        let response = client.get(url).headers(headers).send()?;

        println!("{:?}", response.text()?);
        Ok(())
    }
}

use futures_util::{future, pin_mut, StreamExt};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_tungstenite::{connect_async, tungstenite::{Message, client::IntoClientRequest}};
use url::Url;

#[tokio::main]
async fn main() {
    let token = "Bearer eyJhbGciOiJSUzI1NiIsImtpZCI6Ik1ySnFPV3FHdlVGNFRaTUxLeUhHeWdxUUlGN3c5Uzc1STlacmxTaHMxNlUifQ.eyJhdWQiOlsiaHR0cHM6Ly9rdWJlcm5ldGVzLmRlZmF1bHQuc3ZjLmNsdXN0ZXIubG9jYWwiXSwiZXhwIjoxNzMyNzg0Mzc4LCJpYXQiOjE3MDEyNDgzNzgsImlzcyI6Imh0dHBzOi8va3ViZXJuZXRlcy5kZWZhdWx0LnN2Yy5jbHVzdGVyLmxvY2FsIiwia3ViZXJuZXRlcy5pbyI6eyJuYW1lc3BhY2UiOiJpZHAiLCJwb2QiOnsibmFtZSI6IndlYi10ZXJtIiwidWlkIjoiZWQxNzgwNzYtMmFkNy00MmE3LWIwZjktZDVlOWI0ZWYzNzhiIn0sInNlcnZpY2VhY2NvdW50Ijp7Im5hbWUiOiJhZG1pbiIsInVpZCI6IjgwYjBkZDJmLTNmNjYtNDNhYS04Y2Y2LTdiYjAzMjYyNDdmMyJ9LCJ3YXJuYWZ0ZXIiOjE3MDEyNTE5ODV9LCJuYmYiOjE3MDEyNDgzNzgsInN1YiI6InN5c3RlbTpzZXJ2aWNlYWNjb3VudDppZHA6YWRtaW4ifQ.Kyh5FQ4H-WJBPzt4Ejh1P0MAUqFexOXYn9tbyC3el-ZWLhUMi_vHlTkxFCWRCCN9rDusuxf5O3B2-fxGb0wqKSqLCyVwq-p_TCl6wDyscEswEw6TglJu_lObfplNa-YbcZQQD1G_5DB6i87ZKP8JHxvwe7Jp1RzXyGz1XzOjb--16VLvIZ16DXpkgcBCGKMHPN9zWCZgnoicfGOFFOj7ws6RoG8TiwnB7IBFrFESx9HiZLW0lU0POYeHuNWvlorqp9KiavAG1Yeu2Yp0zaduiEJanrJhD7z3ihJKLpimasp0AYOOG3WY1CI1xIM6fXETFKunYpD3PLczrAu5_oONPQ";
    let url = "wss://192.168.12.12:6443/api/v1/namespaces/idp/pods/web-term/exec?container=web-term-app&stdin=true&stdout=true&stderr=true&tty=true&command=bash&pretty=true&follow=true";
    let (stdin_tx, stdin_rx) = futures_channel::mpsc::unbounded();
    tokio::spawn(read_stdin(stdin_tx));

    let url = Url::parse(url).unwrap();
    let mut request = url.into_client_request().unwrap();
    let headers = request.headers_mut();
    headers.insert("Authorization", token.parse().unwrap());
    let (ws_stream, _) = connect_async(request).await.expect("Failed to connect");
    println!("WebSocket handshake has been successfully completed");

    let (write, read) = ws_stream.split();
    let stdin_to_ws = stdin_rx.map(Ok).forward(write);
    let ws_to_stdout = {
        read.for_each(|message| async {
            let data = message.unwrap().into_data();
            tokio::io::stdout().write_all(&data).await.unwrap();
        })
    };

    pin_mut!(stdin_to_ws, ws_to_stdout);
    future::select(stdin_to_ws, ws_to_stdout).await;
}

async fn read_stdin(tx: futures_channel::mpsc::UnboundedSender<Message>) {
    let mut stdin = tokio::io::stdin();
    loop {
        let mut buf = vec![0; 1024];
        let n = match stdin.read(&mut buf).await {
            Err(_) | Ok(0) => break,
            Ok(n) => n,
        };
        buf.truncate(n);
        tx.unbounded_send(Message::binary(buf)).unwrap();
    }
}

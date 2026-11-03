//! Limit HTTP bodies during streaming, including chunked/unknown-length replies.
use serde::de::DeserializeOwned;
pub async fn json_limited<T: DeserializeOwned>(
    mut response: reqwest::Response,
    max: usize,
) -> Result<T, String> {
    if response.content_length().is_some_and(|n| n > max as u64) {
        return Err("La respuesta del servicio es demasiado grande.".into());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "No se pudo leer la respuesta del servicio.")?
    {
        if chunk.len() > max.saturating_sub(bytes.len()) {
            return Err("La respuesta del servicio es demasiado grande.".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes)
        .map_err(|_| "El servicio devolvió una respuesta inválida.".into())
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    async fn reply(raw: &'static str, max: usize) -> Result<serde_json::Value, String> {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let worker = std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            s.set_read_timeout(Some(std::time::Duration::from_secs(3)))
                .unwrap();
            let mut b = [0; 4096];
            let _ = s.read(&mut b);
            let _ = s.write_all(raw.as_bytes());
        });
        let response = reqwest::Client::builder()
            .no_proxy()
            .timeout(std::time::Duration::from_secs(3))
            .build()
            .unwrap()
            .get(format!("http://{addr}"))
            .send()
            .await
            .unwrap();
        let result = json_limited(response, max).await;
        worker.join().unwrap();
        result
    }
    #[tokio::test]
    async fn accepts_exactly_bounded_json_and_rejects_chunked_overflow() {
        assert_eq!(
            reply(
                "HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: 2\r\n\r\n{}",
                2
            )
            .await
            .unwrap(),
            serde_json::json!({})
        );
        assert!(reply("HTTP/1.1 200 OK\r\nConnection: close\r\nTransfer-Encoding: chunked\r\n\r\n4\r\n1234\r\n0\r\n\r\n",3).await.is_err());
        assert!(reply(
            "HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: 1000000000\r\n\r\n",
            1024
        )
        .await
        .unwrap_err()
        .contains("demasiado grande"));
        assert!(reply(
            "HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: 1\r\n\r\nx",
            2
        )
        .await
        .is_err());
    }
}

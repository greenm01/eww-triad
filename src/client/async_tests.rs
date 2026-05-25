use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::json;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;

use super::*;

static NEXT_SOCKET: AtomicU64 = AtomicU64::new(0);

fn socket_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "eww-triad-async-client-{name}-{}-{}.sock",
        std::process::id(),
        NEXT_SOCKET.fetch_add(1, Ordering::Relaxed)
    ))
}

#[tokio::test]
async fn async_client_reads_state_over_fake_socket() {
    let path = socket_path("state");
    let listener = UnixListener::bind(&path).unwrap();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut reader = BufReader::new(stream);
        let mut request = String::new();
        reader.read_line(&mut request).await.unwrap();
        assert!(request.contains("\"request\":\"state\""));
        let mut stream = reader.into_inner();
        stream
            .write_all(
                json!({"ok": true, "triad": {"version": 1, "type": "state", "state": {"version": 9, "layout": {}, "windows": []}}})
                    .to_string()
                    .as_bytes(),
            )
            .await
            .unwrap();
        stream.write_all(b"\n").await.unwrap();
    });

    let client = AsyncClient::connect(&path);
    let state = client.state_raw().await.unwrap();
    assert_eq!(state["version"], json!(9));
    server.await.unwrap();
    let _ = std::fs::remove_file(path);
}

use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::Sender;

/// Starts a background HTTP IPC server listening for browser extension requests.
pub fn start_ipc_server(tx: Sender<String>, port: u16) {
    tokio::spawn(async move {
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let listener = match TcpListener::bind(addr).await {
            Ok(l) => l,
            Err(_) => return,
        };

        loop {
            let (stream, _) = match listener.accept().await {
                Ok(conn) => conn,
                Err(_) => break,
            };
            let client_tx = tx.clone();
            tokio::spawn(async move {
                handle_client(stream, client_tx).await;
            });
        }
    });
}

/// Handles incoming HTTP request from browser extension or external caller.
async fn handle_client(mut stream: TcpStream, tx: Sender<String>) {
    let mut buffer = [0u8; 4096];
    let n = match stream.read(&mut buffer).await {
        Ok(n) if n > 0 => n,
        _ => return,
    };

    let request_str = String::from_utf8_lossy(&buffer[..n]);
    let mut lines = request_str.lines();
    let request_line = match lines.next() {
        Some(line) => line,
        None => return,
    };

    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return;
    }

    let method = parts[0];
    let path = parts[1];

    let cors_headers = "Access-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\n";

    if method == "OPTIONS" {
        let response = format!("HTTP/1.1 204 No Content\r\n{}\r\n", cors_headers);
        let _ = stream.write_all(response.as_bytes()).await;
        return;
    }

    if method == "GET" && (path == "/health" || path == "/ping") {
        let body = r#"{"status":"online"}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{}\r\n{}",
            body.len(),
            cors_headers,
            body
        );
        let _ = stream.write_all(response.as_bytes()).await;
        return;
    }

    if method == "POST" && path == "/download" {
        if let Some(pos) = request_str.find("\r\n\r\n") {
            let body = &request_str[pos + 4..];
            if let Some(url) = parse_url_from_json(body) {
                let _ = tx.send(url).await;
                let res_body = r#"{"status":"accepted"}"#;
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{}\r\n{}",
                    res_body.len(),
                    cors_headers,
                    res_body
                );
                let _ = stream.write_all(response.as_bytes()).await;
                return;
            }
        }
    }

    let res_body = r#"{"error":"not_found"}"#;
    let response = format!(
        "HTTP/1.1 404 Not Found\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{}\r\n{}",
        res_body.len(),
        cors_headers,
        res_body
    );
    let _ = stream.write_all(response.as_bytes()).await;
}

/// Parses the URL field from an incoming JSON body string.
fn parse_url_from_json(body: &str) -> Option<String> {
    let parsed: serde_json::Value = serde_json::from_str(body).ok()?;
    parsed.get("url").and_then(|v| v.as_str()).map(|s| s.to_string())
}

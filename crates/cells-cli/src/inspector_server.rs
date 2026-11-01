//! Serves a cell's debugger to VS Code, Chrome DevTools, or any other client
//! of the Chrome DevTools Protocol.
//!
//! Debugger clients find a target over HTTP, then connect over WebSocket:
//!
//! ```text
//! GET /json/list          → [{ "webSocketDebuggerUrl": "ws://127.0.0.1:9229/<id>", ... }]
//! GET /json/version       → { "Browser": "cells/0.1.0", "Protocol-Version": "1.3" }
//! GET /<id>  (WebSocket)  → a DevTools protocol session on the cell
//! ```
//!
//! A debugger can read and change anything inside the cell, so this server
//! only listens on the loopback interface and only answers requests whose
//! `Host` header names it, which blocks DNS-rebinding attacks from web
//! pages. The WebSocket path includes a random id, so a page cannot connect
//! without first reading `/json/list`, which the same-origin policy prevents.

use std::collections::hash_map::RandomState;
use std::hash::BuildHasher;
use std::hash::Hasher;
use std::net::SocketAddr;

use cells_runtime::CellInspector;
use futures_util::SinkExt;
use futures_util::StreamExt;
use serde_json::json;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::handshake::derive_accept_key;
use tokio_tungstenite::tungstenite::protocol::Role;

/// The default port, the same one Node and Deno use, so debugger
/// configurations written for them work unchanged.
pub const DEFAULT_PORT: u16 = 9229;

/// Requests larger than this are rejected.
const MAX_REQUEST_HEAD: usize = 16 * 1024;

/// Describes the cell to debugger clients.
pub struct Target {
  pub title: String,
  pub url: String,
}

/// A bound, not yet serving, debugger endpoint.
pub struct InspectorServer {
  listener: std::net::TcpListener,
  addr: SocketAddr,
  id: String,
}

impl InspectorServer {
  /// Binds to `127.0.0.1:<port>`. Binding happens before the cell starts so a
  /// busy port is reported immediately.
  pub fn bind(port: u16) -> Result<Self, String> {
    let listener = std::net::TcpListener::bind(("127.0.0.1", port))
      .map_err(|e| format!("could not listen for a debugger on 127.0.0.1:{port}: {e}"))?;
    listener
      .set_nonblocking(true)
      .map_err(|e| format!("could not configure the debugger socket: {e}"))?;
    let addr = listener.local_addr().map_err(|e| e.to_string())?;
    Ok(Self {
      listener,
      addr,
      id: random_id(),
    })
  }

  pub fn websocket_url(&self) -> String {
    format!("ws://{}/{}", self.addr, self.id)
  }

  /// Starts accepting debugger connections. Must be called from inside a
  /// tokio runtime.
  pub fn serve(self, inspector: CellInspector, target: Target) -> Result<(), String> {
    let listener = TcpListener::from_std(self.listener).map_err(|e| e.to_string())?;
    let state = std::sync::Arc::new(Server {
      addr: self.addr,
      id: self.id,
      inspector,
      target,
    });
    tokio::spawn(async move {
      loop {
        let Ok((stream, _)) = listener.accept().await else {
          continue;
        };
        let state = state.clone();
        tokio::spawn(async move {
          let _ = state.handle(stream).await;
        });
      }
    });
    Ok(())
  }
}

struct Server {
  addr: SocketAddr,
  id: String,
  inspector: CellInspector,
  target: Target,
}

struct Request {
  path: String,
  host: Option<String>,
  websocket_key: Option<String>,
}

impl Server {
  async fn handle(&self, mut stream: TcpStream) -> std::io::Result<()> {
    let Some(request) = read_request(&mut stream).await? else {
      return respond(&mut stream, "400 Bad Request", "text/plain", "bad request").await;
    };
    if !self.host_allowed(request.host.as_deref()) {
      return respond(&mut stream, "403 Forbidden", "text/plain", "host not allowed").await;
    }

    if let Some(key) = &request.websocket_key {
      if request.path.trim_start_matches('/') != self.id {
        return respond(&mut stream, "404 Not Found", "text/plain", "unknown target").await;
      }
      let accept = derive_accept_key(key.as_bytes());
      stream
        .write_all(
          format!(
            "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n\r\n"
          )
          .as_bytes(),
        )
        .await?;
      let socket = WebSocketStream::from_raw_socket(stream, Role::Server, None).await;
      self.pump(socket).await;
      return Ok(());
    }

    let ws_host = format!("{}/{}", self.addr, self.id);
    match request.path.split('?').next().unwrap_or("") {
      "/json" | "/json/list" => {
        let body = json!([{
          "description": "cells cell",
          "devtoolsFrontendUrl": format!(
            "devtools://devtools/bundled/js_app.html?experiments=true&v8only=true&ws={ws_host}"
          ),
          "faviconUrl": "",
          "id": self.id,
          "title": self.target.title,
          "type": "node",
          "url": self.target.url,
          "webSocketDebuggerUrl": format!("ws://{ws_host}"),
        }]);
        respond(&mut stream, "200 OK", "application/json", &body.to_string()).await
      }
      "/json/version" => {
        let body = json!({
          "Browser": format!("cells/{}", env!("CARGO_PKG_VERSION")),
          "Protocol-Version": "1.3",
        });
        respond(&mut stream, "200 OK", "application/json", &body.to_string()).await
      }
      _ => respond(&mut stream, "404 Not Found", "text/plain", "not found").await,
    }
  }

  /// Accepts only `Host` headers that name this server's loopback address.
  fn host_allowed(&self, host: Option<&str>) -> bool {
    host.is_some_and(|host| host_allowed(host, self.addr.port()))
  }

  /// Relays protocol messages between the WebSocket and the cell until
  /// either side closes.
  async fn pump(&self, socket: WebSocketStream<TcpStream>) {
    let Some(session) = self.inspector.connect() else {
      return;
    };
    let (to_cell, mut from_cell) = session.split();
    let (mut ws_tx, mut ws_rx) = socket.split();
    loop {
      tokio::select! {
        incoming = ws_rx.next() => match incoming {
          Some(Ok(Message::Text(text))) => {
            if !to_cell.send(text.to_string()) {
              break;
            }
          }
          Some(Ok(Message::Ping(data))) => {
            let _ = ws_tx.send(Message::Pong(data)).await;
          }
          Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
          Some(Ok(_)) => {}
        },
        outgoing = from_cell.recv() => match outgoing {
          Some(text) => {
            if ws_tx.send(Message::Text(text.into())).await.is_err() {
              break;
            }
          }
          // The cell exited.
          None => break,
        },
      }
    }
    let _ = ws_tx.send(Message::Close(None)).await;
  }
}

/// Whether a `Host` header value names the loopback interface on `port`.
fn host_allowed(host: &str, port: u16) -> bool {
  let (name, host_port) = match host.strip_prefix('[') {
    // An IPv6 literal, such as `[::1]:9229`.
    Some(rest) => match rest.split_once(']') {
      Some((ip, after)) => (ip, after.strip_prefix(':')),
      None => return false,
    },
    None => match host.split_once(':') {
      Some((name, p)) => (name, Some(p)),
      None => (host, None),
    },
  };
  matches!(name, "127.0.0.1" | "localhost" | "::1")
    && host_port.is_none_or(|p| p == port.to_string())
}

/// Reads an HTTP request head. Returns `None` if it is malformed.
async fn read_request(stream: &mut TcpStream) -> std::io::Result<Option<Request>> {
  let mut buf = Vec::new();
  let mut chunk = [0u8; 2048];
  while !buf.windows(4).any(|w| w == b"\r\n\r\n") {
    let n = stream.read(&mut chunk).await?;
    if n == 0 || buf.len() + n > MAX_REQUEST_HEAD {
      return Ok(None);
    }
    buf.extend_from_slice(&chunk[..n]);
  }
  let Ok(head) = std::str::from_utf8(&buf) else {
    return Ok(None);
  };
  let mut lines = head.split("\r\n");
  let mut request_line = lines.next().unwrap_or("").split(' ');
  let (Some("GET"), Some(path)) = (request_line.next(), request_line.next()) else {
    return Ok(None);
  };

  let mut host = None;
  let mut websocket_key = None;
  let mut upgrade = false;
  for line in lines {
    let Some((name, value)) = line.split_once(':') else {
      continue;
    };
    let value = value.trim();
    match name.trim().to_ascii_lowercase().as_str() {
      "host" => host = Some(value.to_string()),
      "upgrade" => upgrade = value.eq_ignore_ascii_case("websocket"),
      "sec-websocket-key" => websocket_key = Some(value.to_string()),
      _ => {}
    }
  }
  Ok(Some(Request {
    path: path.to_string(),
    host,
    websocket_key: websocket_key.filter(|_| upgrade),
  }))
}

async fn respond(
  stream: &mut TcpStream,
  status: &str,
  content_type: &str,
  body: &str,
) -> std::io::Result<()> {
  let response = format!(
    "HTTP/1.1 {status}\r\nContent-Type: {content_type}; charset=UTF-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{body}",
    body.len()
  );
  stream.write_all(response.as_bytes()).await?;
  stream.shutdown().await
}

/// A UUID-shaped identifier from the operating system's random hash keys.
fn random_id() -> String {
  let a = RandomState::new().build_hasher().finish();
  let b = RandomState::new().build_hasher().finish();
  format!(
    "{:08x}-{:04x}-4{:03x}-{:04x}-{:012x}",
    a >> 32,
    (a >> 16) & 0xffff,
    a & 0xfff,
    (b >> 48) & 0x3fff | 0x8000,
    b & 0xffff_ffff_ffff
  )
}

#[cfg(test)]
mod tests {
  use super::host_allowed;
  use super::random_id;

  #[test]
  fn only_loopback_hosts_are_allowed() {
    for ok in ["127.0.0.1:9229", "localhost:9229", "[::1]:9229", "localhost", "[::1]"] {
      assert!(host_allowed(ok, 9229), "{ok}");
    }
    for bad in ["evil.example:9229", "127.0.0.1:80", "127.0.0.1.evil.example", "[::1", ""] {
      assert!(!host_allowed(bad, 9229), "{bad}");
    }
  }

  #[test]
  fn ids_are_unique_and_uuid_shaped() {
    let (a, b) = (random_id(), random_id());
    assert_ne!(a, b);
    assert_eq!(a.len(), 36);
    assert_eq!(a.matches('-').count(), 4);
  }
}

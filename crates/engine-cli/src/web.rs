//! The static file server for the viewer page.
//!
//! The engine serves the page itself, so one program starts the whole viewer, the page's
//! origin is already on the socket server's loopback allowlist, and every response can
//! carry the two cross-origin-isolation headers the page's memory gauge requires. A page
//! opened as a `file://` document cannot work at all: a browser refuses a module script
//! over that scheme, and the page is built from modules.
//!
//! This listener speaks plain HTTP and is deliberately separate from the WebSocket server
//! in the `stream` crate. The socket's origin allowlist and version guard have nothing to
//! do with serving a stylesheet, and a MIME table does not belong beside them.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use anyhow::Context;

/// The longest request line and header block this server reads. A browser's request for a
/// local file is far below it; anything larger is cut off rather than buffered.
const MAX_REQUEST_BYTES: u64 = 8 * 1024;
/// The most connections served at once. A page load asks for about twenty files; a client
/// that opens many more and sends nothing must not pile up threads in the process.
const MAX_CONNECTIONS: usize = 64;
/// How long a connection may take to send its request or read the response.
const IO_TIMEOUT: Duration = Duration::from_secs(10);

/// A running static file server. Dropping the handle leaves the thread serving; the
/// process ends it.
pub struct WebServer {
    port: u16,
}

impl WebServer {
    /// The address a person opens.
    pub fn address(&self) -> String {
        format!("http://127.0.0.1:{}/", self.port)
    }
}

/// The one generated response. A page served on one port cannot guess the WebSocket port
/// on another, and the operating system chooses both at every run, so the server answers
/// for itself rather than making a person paste a number into a query string.
const ENGINE_JSON: &str = "/engine.json";
/// The two actions a page may ask of the launcher. Both change state, so both are POST.
const RESTART: &str = "/engine/restart";
const ABANDON: &str = "/engine/abandon";

/// What a page asks the process that serves it to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Start the engine again from the match's latest snapshot.
    Restart,
    /// Stop the engine and give the match up.
    Abandon,
}

/// Where `/engine.json` comes from, and who answers the two actions.
pub trait Status: Send + Sync {
    /// The `/engine.json` body.
    fn json(&self) -> String;
    /// Carries out one action and returns the new `/engine.json` body, or `None` when this
    /// server takes no actions.
    fn act(&self, action: Action) -> Option<String>;
}

/// The status of a page served by the engine itself: always running on one socket port, and
/// no actions, because nothing survives this process to carry them out.
pub struct Fixed {
    pub socket_port: u16,
    pub match_id: String,
}

impl Status for Fixed {
    fn json(&self) -> String {
        serde_json::json!({
            "engine.state": "running",
            "socket.port": self.socket_port,
            "protocol.version": protocol::PROTOCOL_VERSION,
            "engine.pid": std::process::id(),
            "engine.path": null,
            "engine.reason": null,
            "snapshot.tick": null,
            "match.id": self.match_id,
            "launcher": false,
        })
        .to_string()
    }

    fn act(&self, _action: Action) -> Option<String> {
        None
    }
}

/// Environment variable that names the page folder.
pub const WEB_DIR_ENV: &str = "SM_WEB_DIR";

/// Finds the page folder the way the engine finds its content folder. When `flag` is given,
/// only that folder is used. Otherwise, when `SM_WEB_DIR` is set, only that folder is used.
/// Only when neither is set does the search fall through to `./web`, then the `web` folder
/// beside the running binary, which is where an installed game keeps it. A folder counts
/// only when it holds `index.html`; the refusal lists every folder tried.
pub fn resolve_web_dir(flag: Option<&Path>) -> anyhow::Result<PathBuf> {
    let tried: Vec<PathBuf> = if let Some(dir) = flag {
        vec![dir.to_path_buf()]
    } else if let Some(dir) = std::env::var_os(WEB_DIR_ENV).filter(|d| !d.is_empty()) {
        vec![PathBuf::from(dir)]
    } else {
        let mut tried = vec![PathBuf::from("web")];
        if let Some(dir) = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(Path::to_path_buf))
        {
            tried.push(dir.join("web"));
        }
        tried
    };
    if let Some(found) = tried.iter().find(|dir| dir.join("index.html").is_file()) {
        return Ok(found.clone());
    }
    let list: Vec<String> = tried.iter().map(|p| p.display().to_string()).collect();
    anyhow::bail!(
        "cannot read page folder (tried {}): no folder holds index.html",
        list.join(", ")
    )
}

/// Serves `dir` on a loopback port the operating system chooses. `status` answers
/// `/engine.json`, which tells the page the WebSocket port and the state of the engine.
pub fn start(dir: &Path, status: Arc<dyn Status>) -> anyhow::Result<WebServer> {
    let root = std::fs::canonicalize(dir)
        .with_context(|| format!("cannot serve the page folder {}", dir.display()))?;
    anyhow::ensure!(
        root.is_dir(),
        "the page folder {} is not a folder",
        dir.display()
    );
    let listener = TcpListener::bind("127.0.0.1:0").context("cannot bind a loopback port")?;
    let port = listener
        .local_addr()
        .context("cannot read the bound port")?
        .port();
    let files = count_files(&root);
    tracing::info!(
        signal = "web.serving",
        dir = %shown_dir(dir),
        files,
        isolated = true,
        port
    );

    let served = root.clone();
    let open = Arc::new(AtomicUsize::new(0));
    std::thread::Builder::new()
        .name("web".into())
        .spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                // Past the cap the connection is closed unanswered; a browser retries.
                if open.fetch_add(1, Ordering::AcqRel) >= MAX_CONNECTIONS {
                    open.fetch_sub(1, Ordering::AcqRel);
                    tracing::warn!(signal = "web.connection_refused", open = MAX_CONNECTIONS);
                    continue;
                }
                // A silent or stalled client times out instead of holding its thread.
                if stream.set_read_timeout(Some(IO_TIMEOUT)).is_err()
                    || stream.set_write_timeout(Some(IO_TIMEOUT)).is_err()
                {
                    open.fetch_sub(1, Ordering::AcqRel);
                    continue;
                }
                let root = served.clone();
                let status = Arc::clone(&status);
                let done = Arc::clone(&open);
                // One thread per request. A page load asks for about twenty files, and a
                // serial server would answer them one after another.
                let spawned =
                    std::thread::Builder::new()
                        .name("web-conn".into())
                        .spawn(move || {
                            let _ = answer(stream, &root, port, status.as_ref());
                            done.fetch_sub(1, Ordering::AcqRel);
                        });
                if spawned.is_err() {
                    open.fetch_sub(1, Ordering::AcqRel);
                }
            }
        })
        .context("cannot start the page server thread")?;

    Ok(WebServer { port })
}

/// Reads one request and writes one response.
fn answer(
    mut stream: TcpStream,
    root: &Path,
    page_port: u16,
    status: &dyn Status,
) -> std::io::Result<()> {
    let Some(request) = read_request(&stream) else {
        return write_response(&mut stream, 400, "text/plain", b"bad request", false);
    };
    // A page on another site that rebinds its own name to this machine sends that name as
    // the host. Only the loopback names this server listens on are answered.
    if !host_allowed(request.host.as_deref(), page_port) {
        tracing::warn!(
            signal = "web.host_refused",
            host = %request.host.as_deref().unwrap_or("none")
        );
        return write_response(&mut stream, 421, "text/plain", b"refused host", false);
    }
    let mut parts = request.line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let target = parts.next().unwrap_or_default().to_string();
    let path = target.split(['?', '#']).next().unwrap_or_default();
    if method == "POST" && (path == RESTART || path == ABANDON) {
        // Only the page this server serves may ask. A page on any other site sends its own
        // origin, or none, and is refused before anything happens.
        let own = format!("http://127.0.0.1:{page_port}");
        if request.origin.as_deref() != Some(own.as_str()) {
            tracing::warn!(
                signal = "web.action_refused",
                origin = %request.origin.as_deref().unwrap_or("none"),
                path
            );
            return write_response(&mut stream, 403, "text/plain", b"refused origin", false);
        }
        let action = if path == RESTART {
            Action::Restart
        } else {
            Action::Abandon
        };
        return match status.act(action) {
            Some(body) => write_response(
                &mut stream,
                202,
                "application/json; charset=utf-8",
                body.as_bytes(),
                false,
            ),
            None => write_response(
                &mut stream,
                405,
                "text/plain",
                b"this engine takes no actions; start it with engine-cli launch",
                false,
            ),
        };
    }
    if method != "GET" && method != "HEAD" {
        return write_response(
            &mut stream,
            405,
            "text/plain",
            b"only GET and HEAD are served",
            false,
        );
    }
    let head_only = method == "HEAD";

    if path == ENGINE_JSON {
        let body = status.json();
        return write_response(
            &mut stream,
            200,
            "application/json; charset=utf-8",
            body.as_bytes(),
            head_only,
        );
    }

    let Some(relative) = safe_relative(&target) else {
        return write_response(&mut stream, 403, "text/plain", b"refused path", head_only);
    };
    // The join is checked, not trusted. A symbolic link inside the folder could still
    // point outside it, so the resolved path is compared against the resolved root.
    let Ok(resolved) = std::fs::canonicalize(root.join(&relative)) else {
        return write_response(&mut stream, 404, "text/plain", b"not found", head_only);
    };
    if !resolved.starts_with(root) || !resolved.is_file() {
        return write_response(&mut stream, 403, "text/plain", b"refused path", head_only);
    }
    let Ok(body) = std::fs::read(&resolved) else {
        return write_response(&mut stream, 404, "text/plain", b"not found", head_only);
    };
    write_response(&mut stream, 200, mime_for(&resolved), &body, head_only)
}

/// The request line and the two headers this server reads.
struct Request {
    line: String,
    origin: Option<String>,
    host: Option<String>,
}

/// `true` for a `Host` of `127.0.0.1` or `localhost`, with no port or this server's own.
fn host_allowed(host: Option<&str>, page_port: u16) -> bool {
    let Some(host) = host else {
        return false;
    };
    let (name, port) = match host.rsplit_once(':') {
        Some((name, port)) => (name, Some(port)),
        None => (host, None),
    };
    let local = name == "127.0.0.1" || name.eq_ignore_ascii_case("localhost");
    local && port.is_none_or(|p| p.parse::<u16>().ok() == Some(page_port))
}

/// Reads the request line, then drains the header block, keeping `Origin`.
fn read_request(stream: &TcpStream) -> Option<Request> {
    let mut reader = BufReader::new(stream.try_clone().ok()?.take(MAX_REQUEST_BYTES));
    let mut line = String::new();
    reader.read_line(&mut line).ok()?;
    let line = line.trim_end().to_string();
    if line.is_empty() {
        return None;
    }
    let mut origin = None;
    let mut host = None;
    let mut header = String::new();
    while reader.read_line(&mut header).ok()? > 0 {
        let text = header.trim_end();
        if text.is_empty() {
            break;
        }
        if let Some((name, value)) = text.split_once(':') {
            if name.trim().eq_ignore_ascii_case("origin") {
                origin = Some(value.trim().to_string());
            } else if name.trim().eq_ignore_ascii_case("host") {
                host = Some(value.trim().to_string());
            }
        }
        header.clear();
    }
    Some(Request { line, origin, host })
}

/// Turns a request target into a relative path inside the served folder, or refuses it.
fn safe_relative(target: &str) -> Option<PathBuf> {
    let path = target.split(['?', '#']).next().unwrap_or_default();
    let path = percent_decode(path);
    if !path.starts_with('/') || path.starts_with("//") {
        return None;
    }
    if path.contains('\\') || path.contains(':') || path.contains('\0') {
        return None;
    }
    let mut out = PathBuf::new();
    for part in path.split('/').filter(|p| !p.is_empty()) {
        if part == "." || part == ".." {
            return None;
        }
        out.push(part);
    }
    if out.as_os_str().is_empty() {
        out.push("index.html");
    }
    Some(out)
}

/// Decodes `%XX` escapes. An invalid escape stays as written, so it cannot smuggle a
/// separator past the checks above.
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or_default();
            if let Ok(byte) = u8::from_str_radix(hex, 16) {
                out.push(byte);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The content type for a file, by extension.
fn mime_for(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "html" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        // A module script refused for its content type never runs, and this page is all
        // modules. Python's own static server answers `.mjs` with `text/plain`.
        "mjs" | "js" => "text/javascript; charset=utf-8",
        "woff2" => "font/woff2",
        "json" => "application/json; charset=utf-8",
        "png" => "image/png",
        "svg" => "image/svg+xml",
        _ => "application/octet-stream",
    }
}

/// Writes one response. Every response carries the two isolation headers, because
/// `performance.measureUserAgentSpecificMemory()` is unavailable without them.
fn write_response(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
    head_only: bool,
) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        202 => "Accepted",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        _ => "Method Not Allowed",
    };
    let mut head = String::new();
    head.push_str(&format!("HTTP/1.1 {status} {reason}\r\n"));
    head.push_str(&format!("Content-Type: {content_type}\r\n"));
    head.push_str(&format!("Content-Length: {}\r\n", body.len()));
    head.push_str("Cross-Origin-Opener-Policy: same-origin\r\n");
    head.push_str("Cross-Origin-Embedder-Policy: require-corp\r\n");
    head.push_str("Cache-Control: no-store\r\n");
    head.push_str("Connection: close\r\n\r\n");
    stream.write_all(head.as_bytes())?;
    if !head_only {
        stream.write_all(body)?;
    }
    stream.flush()
}

/// Counts the files under `root`, for the `web.serving` signal.
fn count_files(root: &Path) -> usize {
    let mut count = 0;
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                count += 1;
            }
        }
    }
    count
}

/// The folder as a person named it, never the absolute path.
fn shown_dir(dir: &Path) -> String {
    dir.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| dir.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_loopback_host_on_this_port_is_answered() {
        for host in ["127.0.0.1", "127.0.0.1:8123", "localhost:8123", "LOCALHOST"] {
            assert!(host_allowed(Some(host), 8123), "{host} must be answered");
        }
        for host in [
            "evil.example:8123",
            "127.0.0.1:9999",
            "127.0.0.1.evil.example",
            "",
        ] {
            assert!(!host_allowed(Some(host), 8123), "{host} must be refused");
        }
        assert!(!host_allowed(None, 8123));
    }

    #[test]
    fn a_traversal_or_an_absolute_path_is_refused() {
        assert!(safe_relative("/../secrets.txt").is_none());
        assert!(safe_relative("/web/../../secrets.txt").is_none());
        assert!(safe_relative("/%2e%2e/secrets.txt").is_none());
        assert!(safe_relative("/C:/Windows/win.ini").is_none());
        assert!(safe_relative("/a\\b").is_none());
        assert!(safe_relative("//evil.example/x").is_none());
        assert!(safe_relative("index.html").is_none());
    }

    #[test]
    fn a_plain_path_resolves_and_the_root_becomes_the_index() {
        assert_eq!(safe_relative("/"), Some(PathBuf::from("index.html")));
        assert_eq!(safe_relative("/?v=1"), Some(PathBuf::from("index.html")));
        assert_eq!(
            safe_relative("/tokens.css"),
            Some(PathBuf::from("tokens.css"))
        );
        assert_eq!(
            safe_relative("/fonts/ibm-plex-sans-400.woff2"),
            Some(PathBuf::from("fonts").join("ibm-plex-sans-400.woff2"))
        );
    }

    #[test]
    fn a_module_is_typed_as_javascript() {
        assert_eq!(
            mime_for(Path::new("a/decode.mjs")),
            "text/javascript; charset=utf-8"
        );
        assert_eq!(mime_for(Path::new("a/fonts.woff2")), "font/woff2");
        assert_eq!(
            mime_for(Path::new("a/index.html")),
            "text/html; charset=utf-8"
        );
        assert_eq!(
            mime_for(Path::new("a/fixture.bin")),
            "application/octet-stream"
        );
    }
}

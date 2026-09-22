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

use anyhow::Context;

/// The longest request line and header block this server reads. A browser's request for a
/// local file is far below it; anything larger is cut off rather than buffered.
const MAX_REQUEST_BYTES: u64 = 8 * 1024;

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

/// Serves `dir` on a loopback port the operating system chooses. `socket_port` is the
/// WebSocket port the page connects to, which it reads back from `/engine.json`.
pub fn start(dir: &Path, socket_port: u16) -> anyhow::Result<WebServer> {
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
    std::thread::Builder::new()
        .name("web".into())
        .spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                let root = served.clone();
                // One thread per request. A page load asks for about twenty files, and a
                // serial server would answer them one after another.
                let _ = std::thread::Builder::new()
                    .name("web-conn".into())
                    .spawn(move || {
                        let _ = answer(stream, &root, socket_port);
                    });
            }
        })
        .context("cannot start the page server thread")?;

    Ok(WebServer { port })
}

/// Reads one request and writes one response.
fn answer(mut stream: TcpStream, root: &Path, socket_port: u16) -> std::io::Result<()> {
    let Some(line) = read_request_line(&stream) else {
        return write_response(&mut stream, 400, "text/plain", b"bad request", false);
    };
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let target = parts.next().unwrap_or_default().to_string();
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

    if target.split(['?', '#']).next() == Some(ENGINE_JSON) {
        let body = format!(
            "{{\"socket.port\":{socket_port},\"protocol.version\":{}}}",
            protocol::PROTOCOL_VERSION
        );
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

/// Reads the request line, then drains the header block.
fn read_request_line(stream: &TcpStream) -> Option<String> {
    let mut reader = BufReader::new(stream.try_clone().ok()?.take(MAX_REQUEST_BYTES));
    let mut line = String::new();
    reader.read_line(&mut line).ok()?;
    let line = line.trim_end().to_string();
    if line.is_empty() {
        return None;
    }
    let mut header = String::new();
    while reader.read_line(&mut header).ok()? > 0 {
        if header.trim_end().is_empty() {
            break;
        }
        header.clear();
    }
    Some(line)
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

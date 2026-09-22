//! The static file server that carries the viewer page: it prints an address, refuses a
//! path that climbs out of the folder, types a module as JavaScript, and puts the two
//! cross-origin-isolation headers on every response.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Kills the child whatever happens. Without this a failed assertion leaves the replay
/// running, and the orphan holds the test harness's output pipe open, so the run hangs
/// instead of reporting which assertion failed.
struct ChildGuard(std::process::Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("engine-cli-web-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the test folder is created");
    dir
}

fn bin(data_dir: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_engine-cli"));
    cmd.env("SM_DATA_DIR", data_dir);
    cmd.env(
        "SM_CONTENT_DIR",
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"),
    );
    cmd
}

/// Records a one-minute fixture and returns its path.
fn fixture(dir: &Path) -> PathBuf {
    let path = dir.join("match.smfx");
    let out = bin(dir)
        .args(["record", "--seed", "7", "--minutes", "1", "--out"])
        .arg(&path)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    path
}

/// A page folder with one document and one module.
fn page(dir: &Path) -> PathBuf {
    let web = dir.join("web");
    std::fs::create_dir_all(web.join("sub")).unwrap();
    std::fs::write(web.join("index.html"), "<!doctype html><title>t</title>").unwrap();
    std::fs::write(web.join("decode.mjs"), "export const kind = 1;\n").unwrap();
    std::fs::write(dir.join("secret.txt"), "not served").unwrap();
    web
}

/// One HTTP request, with the whole response as text.
fn get(port: u16, target: &str) -> String {
    let mut socket = TcpStream::connect(("127.0.0.1", port)).unwrap();
    socket
        .write_all(
            format!("GET {target} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
                .as_bytes(),
        )
        .unwrap();
    let mut response = Vec::new();
    socket.read_to_end(&mut response).unwrap();
    String::from_utf8_lossy(&response).into_owned()
}

#[test]
fn a_missing_page_folder_is_refused_naming_the_path() {
    let dir = temp("missing");
    let path = fixture(&dir);
    let absent = dir.join("no-such-folder");
    let out = bin(&dir)
        .args(["replay", "--fixture"])
        .arg(&path)
        .arg("--web")
        .arg(&absent)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("cannot serve the page folder"), "{stderr}");
    assert!(stderr.contains("no-such-folder"), "{stderr}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_page_is_served_with_its_address_printed_and_both_isolation_headers() {
    let dir = temp("serve");
    let path = fixture(&dir);
    let web = page(&dir);

    let child = bin(&dir)
        .args(["replay", "--fixture"])
        .arg(&path)
        .arg("--web")
        .arg(&web)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut child = ChildGuard(child);
    let mut lines = BufReader::new(child.0.stdout.take().unwrap()).lines();

    // The socket port stays the first line, because two other tests parse it as a port.
    let socket_port = lines.next().unwrap().unwrap();
    socket_port
        .trim()
        .parse::<u16>()
        .unwrap_or_else(|e| panic!("replay must print a port, printed {socket_port:?}: {e}"));

    // The page address is the line a reader copies, so it is printed last.
    let address = lines.next().unwrap().unwrap();
    let address = address.trim().to_string();
    assert!(
        address.starts_with("http://127.0.0.1:") && address.ends_with('/'),
        "the page address is printed: {address:?}"
    );
    let port: u16 = address
        .trim_start_matches("http://127.0.0.1:")
        .trim_end_matches('/')
        .parse()
        .unwrap();

    let root = get(port, "/");
    assert!(root.starts_with("HTTP/1.1 200 OK"), "{root}");
    assert!(root.contains("Content-Type: text/html"), "{root}");
    assert!(root.contains("<!doctype html>"), "{root}");

    // The memory gauge needs both headers, and the page needs them on every response.
    for response in [&root, &get(port, "/decode.mjs"), &get(port, "/nothing")] {
        assert!(
            response.contains("Cross-Origin-Opener-Policy: same-origin"),
            "{response}"
        );
        assert!(
            response.contains("Cross-Origin-Embedder-Policy: require-corp"),
            "{response}"
        );
        assert!(response.contains("Cache-Control: no-store"), "{response}");
    }

    // A module refused for its content type never runs, and this page is all modules.
    let module = get(port, "/decode.mjs");
    assert!(module.starts_with("HTTP/1.1 200 OK"), "{module}");
    assert!(module.contains("Content-Type: text/javascript"), "{module}");

    for climb in [
        "/../secret.txt",
        "/sub/../../secret.txt",
        "/%2e%2e/secret.txt",
    ] {
        let refused = get(port, climb);
        assert!(refused.starts_with("HTTP/1.1 403 Forbidden"), "{refused}");
        assert!(!refused.contains("not served"), "{refused}");
    }

    let missing = get(port, "/absent.css");
    assert!(missing.starts_with("HTTP/1.1 404 Not Found"), "{missing}");

    // The page cannot guess the socket port, so the server answers for itself.
    let engine = get(port, "/engine.json");
    assert!(engine.starts_with("HTTP/1.1 200 OK"), "{engine}");
    assert!(
        engine.contains("Content-Type: application/json"),
        "{engine}"
    );
    let body = engine
        .split("\r\n\r\n")
        .nth(1)
        .expect("the response has a body");
    assert!(body.contains("\"protocol.version\":2"), "{body}");
    assert!(
        body.contains(&format!("\"socket.port\":{}", socket_port.trim())),
        "{body} names the socket port {socket_port}"
    );

    let mut socket = TcpStream::connect(("127.0.0.1", port)).unwrap();
    socket
        .write_all(b"DELETE / HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
        .unwrap();
    let mut response = String::new();
    socket.read_to_string(&mut response).unwrap();
    assert!(
        response.starts_with("HTTP/1.1 405 Method Not Allowed"),
        "{response}"
    );

    drop(child);
    let _ = std::fs::remove_dir_all(&dir);
}

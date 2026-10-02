//! The installed layout: the program with `content/` and `web/` beside it, started from an
//! unrelated folder with no path flag and no path variable. The launcher finds the page and
//! the engine finds its content, both beside the program, and the engine's `hello` names
//! the version the program prints.
//!
//! By default the layout is the built program, the repository's content folder and a small
//! generated page folder (an `index.html` and one module), so the test needs no viewer
//! build. When `SM_INSTALL_UNDER_TEST` names a folder, that folder is the layout instead, so
//! the same check runs against an unpacked release archive or a staged package, and two more
//! tests check that its `web/` is the built viewer the release carries and that the previous
//! release's engine ships beside this one in `previous/`, with its own content.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use protocol::ServerMessage;
use serde_json::Value;
use stream::{Client, Incoming};

fn temp(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("engine-cli-install-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the test folder is created");
    dir
}

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Copies `from` into `to`, leaving out any folder named in `skip`.
fn copy_dir(from: &Path, to: &Path, skip: &[&str]) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap().flatten() {
        let path = entry.path();
        let name = entry.file_name();
        if path.is_dir() {
            if !skip.iter().any(|s| name == *s) {
                copy_dir(&path, &to.join(&name), &[]);
            }
        } else {
            std::fs::copy(&path, to.join(&name)).unwrap();
        }
    }
}

/// The program inside a layout.
fn program(install: &Path) -> PathBuf {
    let built = PathBuf::from(env!("CARGO_BIN_EXE_engine-cli"));
    install.join(
        built
            .file_name()
            .expect("the built program has a file name"),
    )
}

/// The layout under test.
fn layout(name: &str) -> PathBuf {
    if let Some(dir) = std::env::var_os("SM_INSTALL_UNDER_TEST").filter(|d| !d.is_empty()) {
        return PathBuf::from(dir);
    }
    let install = temp(name).join("install");
    std::fs::create_dir_all(&install).unwrap();
    std::fs::copy(env!("CARGO_BIN_EXE_engine-cli"), program(&install)).unwrap();
    copy_dir(&repo().join("content"), &install.join("content"), &[]);
    write_page(&install.join("web"));
    install
}

/// A page folder shaped like the built viewer: an `index.html` that loads one module from
/// `assets/`.
fn write_page(web: &Path) {
    std::fs::create_dir_all(web.join("assets")).unwrap();
    std::fs::write(
        web.join("index.html"),
        "<!doctype html>\n<html lang=\"en\">\n  <head>\n    <meta charset=\"utf-8\" />\n    \
         <title>Touchline</title>\n    <script type=\"module\" src=\"/assets/index-test.js\"></script>\n  \
         </head>\n  <body><div id=\"app\"></div></body>\n</html>\n",
    )
    .unwrap();
    std::fs::write(web.join("assets").join("index-test.js"), "export {};\n").unwrap();
}

/// The installed program with no path configuration, run from `cwd`.
fn installed(install: &Path, data: &Path, cwd: &Path) -> Command {
    let mut cmd = Command::new(program(install));
    cmd.current_dir(cwd)
        .env_remove("SM_CONTENT_DIR")
        .env_remove("SM_WEB_DIR")
        .env_remove("SM_ENGINE_PATH")
        .env("SM_DATA_DIR", data)
        .env("SM_LOG", "warn");
    cmd
}

/// Stops the launcher and the worker it names.
struct Running {
    child: Child,
    port: u16,
}

impl Drop for Running {
    fn drop(&mut self) {
        if let Some(pid) = status(self.port)["engine.pid"].as_u64() {
            kill(pid);
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn kill(pid: u64) {
    let pid = pid.to_string();
    let _ = if cfg!(windows) {
        Command::new("taskkill").args(["/F", "/PID", &pid]).output()
    } else {
        Command::new("kill").args(["-9", &pid]).output()
    };
}

/// One HTTP GET; the status code and the body.
fn get(port: u16, target: &str) -> (u16, String) {
    let (code, _, body) = get_with_headers(port, target);
    (code, body)
}

/// One HTTP GET; the status code, the header block and the body.
fn get_with_headers(port: u16, target: &str) -> (u16, String, String) {
    let Ok(mut socket) = TcpStream::connect(("127.0.0.1", port)) else {
        return (0, String::new(), String::new());
    };
    let request = format!("GET {target} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n");
    if socket.write_all(request.as_bytes()).is_err() {
        return (0, String::new(), String::new());
    }
    let mut response = Vec::new();
    let _ = socket.read_to_end(&mut response);
    let text = String::from_utf8_lossy(&response).into_owned();
    let code = text
        .split_whitespace()
        .nth(1)
        .and_then(|c| c.parse().ok())
        .unwrap_or(0);
    let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
    (code, head.to_string(), body.to_string())
}

fn status(port: u16) -> Value {
    serde_json::from_str(&get(port, "/engine.json").1).unwrap_or(Value::Null)
}

/// The second word of `--version`.
fn version_of(program: &Path) -> String {
    let out = Command::new(program).arg("--version").output().unwrap();
    assert!(out.status.success(), "--version fails");
    String::from_utf8_lossy(&out.stdout)
        .split_whitespace()
        .nth(1)
        .expect("--version prints a name and a version")
        .to_string()
}

#[test]
fn the_installed_program_serves_the_page_and_the_engine_without_configuration() {
    let install = layout("found");
    let scratch = temp("found-run");
    let (data, cwd) = (scratch.join("data"), scratch.join("elsewhere"));
    std::fs::create_dir_all(&cwd).unwrap();

    let mut child = installed(&install, &data, &cwd)
        .args(["launch", "--no-start-screen", "--minutes", "1"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().expect("the launcher prints"))
        .read_line(&mut line)
        .unwrap();
    let port: u16 = line
        .trim()
        .trim_start_matches("http://127.0.0.1:")
        .trim_end_matches('/')
        .parse()
        .unwrap_or_else(|e| panic!("launch must print the page address, printed {line:?}: {e}"));
    let running = Running { child, port };

    let (code, body) = get(port, "/");
    assert_eq!(code, 200, "the page is served");
    let index = std::fs::read_to_string(install.join("web").join("index.html")).unwrap();
    assert_eq!(body, index, "the page is the installed index.html");

    let deadline = Instant::now() + Duration::from_secs(60);
    let now = loop {
        let now = status(port);
        if now["engine.state"] == "running" && now["socket.port"].is_u64() {
            break now;
        }
        assert!(
            Instant::now() < deadline,
            "the engine never ran; last status {now}"
        );
        std::thread::sleep(Duration::from_millis(50));
    };
    let socket = u16::try_from(now["socket.port"].as_u64().unwrap()).unwrap();

    let mut client =
        Client::connect(socket, protocol::PROTOCOL_VERSION, "http://127.0.0.1").unwrap();
    let Incoming::Message(message) = client.read().unwrap() else {
        panic!("the first frame is the hello");
    };
    let ServerMessage::Hello(hello) = *message else {
        panic!("the first message is the hello");
    };
    assert_eq!(
        hello.engine_version,
        version_of(&program(&install)),
        "the hello names the version the installed program prints"
    );
    drop(client);
    drop(running);
    let _ = std::fs::remove_dir_all(&scratch);
    if std::env::var_os("SM_INSTALL_UNDER_TEST").is_none_or(|d| d.is_empty()) {
        let _ = std::fs::remove_dir_all(install.parent().unwrap());
    }
}

#[test]
fn a_layout_without_the_page_folder_is_refused_naming_both_folders_tried() {
    if std::env::var_os("SM_INSTALL_UNDER_TEST").is_some_and(|d| !d.is_empty()) {
        // A packaged folder is never changed by a test.
        return;
    }
    let install = layout("no-page");
    std::fs::remove_dir_all(install.join("web")).unwrap();
    let scratch = temp("no-page-run");
    let (data, cwd) = (scratch.join("data"), scratch.join("elsewhere"));
    std::fs::create_dir_all(&cwd).unwrap();

    let out = installed(&install, &data, &cwd)
        .args(["launch", "--no-start-screen", "--minutes", "1"])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(1),
        "a missing page folder is an error"
    );
    assert!(out.stdout.is_empty(), "no address is printed");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("tried viewer/dist, "),
        "the refusal names the built viewer in a checkout: {stderr}"
    );
    assert!(
        stderr.contains(&install.join("web").display().to_string()),
        "the refusal names the folder beside the program: {stderr}"
    );
    let _ = std::fs::remove_dir_all(install.parent().unwrap());
    let _ = std::fs::remove_dir_all(&scratch);
}

/// The licence texts the release carries beside the fonts, one per typeface family.
const LICENCES: [&str; 4] = [
    "OFL-Saira.txt",
    "OFL-Saira-Semi-Condensed.txt",
    "OFL-Barlow.txt",
    "OFL-IBM-Plex.txt",
];

/// What a packaged `web/` must be: the built viewer. Every failure is listed.
fn packaged_page_faults(install: &Path, port: u16) -> Vec<String> {
    let web = install.join("web");
    let mut faults = Vec::new();
    let index = std::fs::read_to_string(web.join("index.html")).unwrap_or_default();
    let module = index
        .split("<script type=\"module\"")
        .nth(1)
        .and_then(|rest| rest.split("src=\"").nth(1))
        .and_then(|rest| rest.split('"').next())
        .map(str::to_string);
    match module {
        Some(src) if src.trim_start_matches('/').starts_with("assets/") => {
            let (code, head, _) =
                get_with_headers(port, &format!("/{}", src.trim_start_matches('/')));
            if code != 200 {
                faults.push(format!("GET {src} answers {code}"));
            }
            if !head
                .to_ascii_lowercase()
                .contains("content-type: text/javascript")
            {
                faults.push(format!("{src} is not served as JavaScript: {head}"));
            }
        }
        Some(src) => faults.push(format!(
            "index.html loads its module from {src}, not assets/"
        )),
        None => faults.push("index.html loads no module script".to_string()),
    }
    for licence in LICENCES {
        if !web.join("fonts").join(licence).is_file() {
            faults.push(format!("web/fonts/{licence} is missing"));
        }
    }
    for left_out in ["src", "tests", "node_modules", "shell-test.html"] {
        if web.join(left_out).exists() {
            faults.push(format!("web/{left_out} is in the package"));
        }
    }
    faults
}

/// The previous release the package carries, from the pin the build scripts read.
fn pinned_previous_version() -> String {
    let pin: Value = serde_json::from_str(include_str!("../../../packaging/previous-engine.json"))
        .expect("the pin is JSON");
    pin["version"]
        .as_str()
        .expect("the pin names a version")
        .to_string()
}

/// What a packaged `previous/` must be: the pinned release's program, which prints its
/// version, with that release's content folder beside it. Every failure is listed.
fn previous_engine_faults(install: &Path) -> Vec<String> {
    let previous = install.join("previous");
    let program = previous.join(program(install).file_name().unwrap());
    let mut faults = Vec::new();
    if !program.is_file() {
        faults.push(format!("{} is missing", program.display()));
    } else {
        let pinned = pinned_previous_version();
        let printed = version_of(&program);
        if printed != pinned {
            faults.push(format!(
                "previous/ holds version {printed}, not the pinned {pinned}"
            ));
        }
        if printed == version_of(&self::program(install)) {
            faults.push(format!(
                "previous/ holds this release's own program ({printed})"
            ));
        }
    }
    if !previous.join("content").join("attributes.json").is_file() {
        faults.push("previous/content/attributes.json is missing".to_string());
    }
    faults
}

/// Starts the installed launcher from an unrelated folder and returns it with its port.
fn launch(install: &Path, scratch: &Path) -> Running {
    let (data, cwd) = (scratch.join("data"), scratch.join("elsewhere"));
    std::fs::create_dir_all(&cwd).unwrap();
    let mut child = installed(install, &data, &cwd)
        .args(["launch", "--no-start-screen", "--minutes", "1"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().expect("the launcher prints"))
        .read_line(&mut line)
        .unwrap();
    let port: u16 = line
        .trim()
        .trim_start_matches("http://127.0.0.1:")
        .trim_end_matches('/')
        .parse()
        .unwrap_or_else(|e| panic!("launch must print the page address, printed {line:?}: {e}"));
    Running { child, port }
}

#[test]
fn a_packaged_layout_carries_the_built_viewer_and_its_font_licences() {
    let Some(dir) = std::env::var_os("SM_INSTALL_UNDER_TEST").filter(|d| !d.is_empty()) else {
        // Only a packaged folder carries the release build.
        return;
    };
    let install = PathBuf::from(dir);
    let scratch = temp("packaged-run");
    let running = launch(&install, &scratch);
    let faults = packaged_page_faults(&install, running.port);
    drop(running);
    let _ = std::fs::remove_dir_all(&scratch);
    assert!(
        faults.is_empty(),
        "the packaged web/ is not the built viewer: {faults:#?}"
    );
}

#[test]
fn the_packaged_check_fails_on_a_page_folder_that_is_not_the_build() {
    if std::env::var_os("SM_INSTALL_UNDER_TEST").is_some_and(|d| !d.is_empty()) {
        // A packaged folder is never changed by a test.
        return;
    }
    // The control: a layout whose web/ holds only an index.html with no module.
    let install = layout("not-the-build");
    std::fs::remove_dir_all(install.join("web")).unwrap();
    std::fs::create_dir_all(install.join("web")).unwrap();
    std::fs::write(
        install.join("web").join("index.html"),
        "<!doctype html><title>x</title>\n",
    )
    .unwrap();
    let scratch = temp("not-the-build-run");
    let running = launch(&install, &scratch);
    let faults = packaged_page_faults(&install, running.port);
    drop(running);
    let _ = std::fs::remove_dir_all(install.parent().unwrap());
    let _ = std::fs::remove_dir_all(&scratch);
    assert!(
        faults.iter().any(|f| f.contains("no module script")),
        "the check must fail on a page folder with no module: {faults:#?}"
    );
    assert!(
        faults.iter().any(|f| f.contains("OFL-Saira.txt")),
        "the check must fail on a page folder with no licence texts: {faults:#?}"
    );
}

#[test]
fn a_packaged_layout_carries_the_previous_release_engine() {
    let Some(dir) = std::env::var_os("SM_INSTALL_UNDER_TEST").filter(|d| !d.is_empty()) else {
        // Only a packaged folder carries the previous release's engine.
        return;
    };
    let faults = previous_engine_faults(&PathBuf::from(dir));
    assert!(
        faults.is_empty(),
        "the package does not carry the previous release's engine: {faults:#?}"
    );
}

#[test]
fn the_previous_engine_check_fails_on_a_layout_without_it() {
    if std::env::var_os("SM_INSTALL_UNDER_TEST").is_some_and(|d| !d.is_empty()) {
        // A packaged folder is never changed by a test.
        return;
    }
    // The control: a layout with no previous/, then one whose previous/ holds this program.
    let install = layout("no-previous");
    let missing = previous_engine_faults(&install);
    let previous = install.join("previous");
    std::fs::create_dir_all(&previous).unwrap();
    std::fs::copy(
        program(&install),
        previous.join(program(&install).file_name().unwrap()),
    )
    .unwrap();
    let own = previous_engine_faults(&install);
    let _ = std::fs::remove_dir_all(install.parent().unwrap());
    assert!(
        missing.iter().any(|f| f.contains("is missing")),
        "the check must fail on a layout with no previous/: {missing:#?}"
    );
    assert!(
        own.iter().any(|f| f.contains("not the pinned")),
        "the check must fail on a previous/ that is not the pinned release: {own:#?}"
    );
    assert!(
        own.iter().any(|f| f.contains("content")),
        "the check must fail on a previous/ with no content: {own:#?}"
    );
}

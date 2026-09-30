//! The web examples' server: static files from the repository, on
//! localhost, and the browser opened on the page asked for.
//!
//!     owlosui-httpd <root> [page] [--no-open]
//!
//! A page loaded from `file://` may not load a WebAssembly module, so the
//! examples need some web server, and this is the one `RUN.CMD` starts:
//! nothing to install beyond the Rust that builds the core anyway. It
//! serves files and nothing else - GET, the MIME types these pages use
//! (the browser insists on `application/wasm` for a module), no caching,
//! so a page edited and reloaded is the page as edited. Only 127.0.0.1
//! can reach it.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Component, Path, PathBuf};

fn main() {
    // `--no-open` serves without starting a browser: for a test, or a
    // machine where the page is opened some other way.
    let all: Vec<String> = std::env::args().skip(1).collect();
    let open_browser = !all.iter().any(|a| a == "--no-open");
    let mut args = all.into_iter().filter(|a| a != "--no-open");
    let root = PathBuf::from(args.next().unwrap_or_else(|| ".".into()));
    let page = args.next().unwrap_or_else(|| "/".into());
    let root = root.canonicalize().unwrap_or_else(|e| fail(&format!("{}: {e}", root.display())));

    // The first free port from 8765: a second RUN.CMD beside the first
    // gets a server of its own rather than an error.
    let (listener, port) = (8765..8800)
        .find_map(|p| TcpListener::bind(("127.0.0.1", p)).ok().map(|l| (l, p)))
        .unwrap_or_else(|| fail("no free port between 8765 and 8799"));

    let url = format!("http://localhost:{port}{page}");
    // canonicalize() on Windows answers `\\?\C:\...`; people write `C:\...`.
    let shown = root.display().to_string();
    println!("Serving {} at http://localhost:{port}/", shown.trim_start_matches(r"\\?\"));
    println!("Ctrl+C stops the server.");
    if open_browser {
        println!("Opening {url}");
        open(&url);
    }

    for stream in listener.incoming().flatten() {
        let root = root.clone();
        std::thread::spawn(move || {
            let _ = serve(stream, &root);
        });
    }
}

fn fail(what: &str) -> ! {
    eprintln!("owlosui-httpd: {what}");
    std::process::exit(1)
}

/// The system's browser, on the page.
fn open(url: &str) {
    #[cfg(windows)]
    let r = std::process::Command::new("cmd").args(["/c", "start", "", url]).spawn();
    #[cfg(target_os = "macos")]
    let r = std::process::Command::new("open").arg(url).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let r = std::process::Command::new("xdg-open").arg(url).spawn();
    if r.is_err() {
        println!("Open it in a browser yourself: {url}");
    }
}

fn serve(mut stream: TcpStream, root: &Path) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    // The headers are read and forgotten: nothing here depends on them.
    loop {
        let mut h = String::new();
        if reader.read_line(&mut h)? <= 2 {
            break;
        }
    }
    let mut parts = line.split_whitespace();
    let (method, target) = (parts.next().unwrap_or(""), parts.next().unwrap_or("/"));
    if method != "GET" && method != "HEAD" {
        return reply(&mut stream, "405 Method Not Allowed", "text/plain", b"GET only\n", true);
    }

    let path = decode(target.split(['?', '#']).next().unwrap_or("/"));
    // Nothing outside the root: every piece of the path must be a plain name.
    let rel = Path::new(path.trim_start_matches('/'));
    if rel.components().any(|c| !matches!(c, Component::Normal(_))) {
        return reply(&mut stream, "403 Forbidden", "text/plain", b"outside the root\n", true);
    }
    let mut file = root.join(rel);
    if file.is_dir() {
        if !path.ends_with('/') {
            // `/Examples/Web` must become `/Examples/Web/`, or the page's
            // relative links resolve one folder too high.
            let head = format!("HTTP/1.1 301 Moved Permanently\r\nLocation: {path}/\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
            return stream.write_all(head.as_bytes());
        }
        file.push("index.html");
    }
    match std::fs::read(&file) {
        Ok(body) => reply(&mut stream, "200 OK", mime(&file), &body, method == "GET"),
        Err(_) => reply(&mut stream, "404 Not Found", "text/plain", format!("no {path}\n").as_bytes(), true),
    }
}

fn reply(stream: &mut TcpStream, status: &str, mime: &str, body: &[u8], with_body: bool) -> std::io::Result<()> {
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes())?;
    if with_body {
        stream.write_all(body)?;
    }
    stream.flush()
}

fn mime(file: &Path) -> &'static str {
    match file.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase().as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json",
        "wasm" => "application/wasm",
        "png" => "image/png",
        "svg" => "image/svg+xml",
        "md" | "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

/// `%20` and friends back into bytes.
fn decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            let hex = std::str::from_utf8(&b[i + 1..i + 3]).ok();
            if let Some(v) = hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

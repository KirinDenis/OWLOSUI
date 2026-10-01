//! The web examples' server: static files from the repository, a folder of
//! files a page may read and save, on localhost, and the browser opened on
//! the page asked for.
//!
//!     owlosui-httpd <root> [page] [--files <folder>] [--no-open]
//!
//! A page loaded from `file://` may not load a WebAssembly module, so the
//! examples need some web server, and this is the one `RUN.CMD` starts:
//! nothing to install beyond the Rust that builds the core anyway. Only
//! 127.0.0.1 can reach it, and nothing is cached, so a page edited and
//! reloaded is the page as edited.
//!
//! What it serves:
//!
//!  * `/...` - the repository's files, read-only, with the MIME types the
//!    pages use (the browser insists on `application/wasm` for a module);
//!    a folder with `?list` gives its names as JSON, so a page can browse
//!    the examples' sources;
//!  * `/files/...` - the `--files` folder the plain way: `GET` a file, `PUT`
//!    one to save it, `GET` a folder with `?list` for its names as JSON;
//!  * `/dav/...` - the same folder over WebDAV (`OPTIONS`, `PROPFIND`,
//!    `GET`, `PUT`), the protocol NAS boxes and Nextcloud share folders
//!    with. The demo's File menu reaches the folder both ways.
//!
//! Both ways into the folder also take WebDAV's four verbs for changing
//! it - `DELETE` a file or a folder, `MKCOL` a new folder, `MOVE` and
//! `COPY` to the place a `Destination` header names - which is what the
//! demo's file manager does to it.
//!
//! Nothing outside the `--files` folder can be written, and nothing outside
//! the root can be read: every piece of a path must be a plain name.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Component, Path, PathBuf};
use std::time::UNIX_EPOCH;

/// The most a PUT may carry: plenty for a text file, not a disk's worth.
const MAX_BODY: usize = 4 * 1024 * 1024;

struct Site {
    root: PathBuf,
    files: Option<PathBuf>,
}

fn main() {
    let mut open_browser = true;
    let mut files = None;
    let mut positional = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            // Serve without starting a browser: for a test, or a machine
            // where the page is opened some other way.
            "--no-open" => open_browser = false,
            "--files" => files = args.next().map(PathBuf::from),
            _ => positional.push(a),
        }
    }
    let mut positional = positional.into_iter();
    let root = PathBuf::from(positional.next().unwrap_or_else(|| ".".into()));
    let page = positional.next().unwrap_or_else(|| "/".into());
    let root = root.canonicalize().unwrap_or_else(|e| fail(&format!("{}: {e}", root.display())));
    let files = files.map(|f| {
        std::fs::create_dir_all(&f).unwrap_or_else(|e| fail(&format!("{}: {e}", f.display())));
        f.canonicalize().unwrap_or_else(|e| fail(&format!("{}: {e}", f.display())))
    });

    // The first free port from 8765: a second RUN.CMD beside the first
    // gets a server of its own rather than an error.
    let (listener, port) = (8765..8800)
        .find_map(|p| TcpListener::bind(("127.0.0.1", p)).ok().map(|l| (l, p)))
        .unwrap_or_else(|| fail("no free port between 8765 and 8799"));

    let url = format!("http://localhost:{port}{page}");
    println!("Serving {} at http://localhost:{port}/", shown(&root));
    if let Some(f) = &files {
        println!("Files a page may read and save: {} at /files/ and, over WebDAV, /dav/", shown(f));
    }
    println!("Ctrl+C stops the server.");
    if open_browser {
        println!("Opening {url}");
        open(&url);
    }

    let site = std::sync::Arc::new(Site { root, files });
    for stream in listener.incoming().flatten() {
        let site = site.clone();
        std::thread::spawn(move || {
            let _ = serve(stream, &site);
        });
    }
}

/// canonicalize() on Windows answers `\\?\C:\...`; people write `C:\...`.
fn shown(p: &Path) -> String {
    p.display().to_string().trim_start_matches(r"\\?\").to_string()
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

struct Request {
    method: String,
    path: String,
    query: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

fn read_request(stream: &TcpStream) -> std::io::Result<Request> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let mut headers = HashMap::new();
    loop {
        let mut h = String::new();
        if reader.read_line(&mut h)? <= 2 {
            break;
        }
        if let Some((k, v)) = h.split_once(':') {
            headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
        }
    }
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let target = parts.next().unwrap_or("/");
    let (path, query) = match target.split_once('?') {
        Some((p, q)) => (p, q),
        None => (target, ""),
    };
    let len: usize = headers.get("content-length").and_then(|v| v.parse().ok()).unwrap_or(0);
    let mut body = vec![0u8; len.min(MAX_BODY)];
    if len > 0 {
        reader.read_exact(&mut body)?;
    }
    Ok(Request { method, path: decode(path), query: query.to_string(), headers, body })
}

fn serve(mut stream: TcpStream, site: &Site) -> std::io::Result<()> {
    let req = read_request(&stream)?;
    if req.headers.get("content-length").and_then(|v| v.parse::<usize>().ok()).unwrap_or(0) > MAX_BODY {
        return reply(&mut stream, "413 Payload Too Large", "text/plain", b"too big\n", true);
    }
    // Every piece of the path must be a plain name: nothing above the root.
    let rel = PathBuf::from(req.path.trim_start_matches('/'));
    if rel.components().any(|c| !matches!(c, Component::Normal(_))) {
        return reply(&mut stream, "403 Forbidden", "text/plain", b"outside the root\n", true);
    }
    let area = |prefix: &str| -> Option<PathBuf> {
        let r = rel.strip_prefix(prefix).ok()?;
        site.files.as_ref().map(|f| f.join(r))
    };
    let in_files = req.path == "/files" || req.path.starts_with("/files/");
    let in_dav = req.path == "/dav" || req.path.starts_with("/dav/");
    if in_files || in_dav {
        let Some(file) = area(if in_files { "files" } else { "dav" }) else {
            return reply(&mut stream, "404 Not Found", "text/plain", b"no --files folder\n", true);
        };
        if matches!(req.method.as_str(), "DELETE" | "MKCOL" | "MOVE" | "COPY") {
            return change(&mut stream, &req, &file, site);
        }
        if in_files { files(&mut stream, &req, &file) } else { dav(&mut stream, &req, &file) }
    } else {
        statics(&mut stream, &req, &site.root.join(&rel))
    }
}

/// `DELETE`, `MKCOL`, `MOVE`, `COPY` on the files folder: WebDAV's verbs
/// for changing it, taken on `/files/` too. A `Destination` is a URL or a
/// path into the same folder, by either name; nothing outside it.
fn change(stream: &mut TcpStream, req: &Request, path: &Path, site: &Site) -> std::io::Result<()> {
    let top = site.files.as_ref().expect("checked by the caller");
    if path == top.as_path() {
        return reply(stream, "403 Forbidden", "text/plain", b"not the folder itself\n", true);
    }
    let done = |s: &mut TcpStream, r: std::io::Result<()>, ok: &str| match r {
        Ok(()) => reply(s, ok, "text/plain", b"", true),
        Err(e) => reply(s, "409 Conflict", "text/plain", format!("{e}\n").as_bytes(), true),
    };
    match req.method.as_str() {
        "DELETE" => {
            if !path.exists() {
                return reply(stream, "404 Not Found", "text/plain", b"no such file\n", true);
            }
            let r = if path.is_dir() { std::fs::remove_dir_all(path) } else { std::fs::remove_file(path) };
            done(stream, r, "204 No Content")
        }
        "MKCOL" => {
            if path.exists() {
                return reply(stream, "405 Method Not Allowed", "text/plain", b"it is there already\n", true);
            }
            done(stream, std::fs::create_dir(path), "201 Created")
        }
        _ => {
            let Some(to) = destination(req, top) else {
                return reply(stream, "400 Bad Request", "text/plain", b"a Destination into the folder, please\n", true);
            };
            if !path.exists() {
                return reply(stream, "404 Not Found", "text/plain", b"no such file\n", true);
            }
            if to.starts_with(path) {
                return reply(stream, "409 Conflict", "text/plain", b"a folder cannot go inside itself\n", true);
            }
            let overwrite = req.headers.get("overwrite").map_or(true, |v| v != "F");
            if to.exists() {
                if !overwrite {
                    return reply(stream, "412 Precondition Failed", "text/plain", b"it is there already\n", true);
                }
                let _ = if to.is_dir() { std::fs::remove_dir_all(&to) } else { std::fs::remove_file(&to) };
            }
            let r = if req.method == "MOVE" { std::fs::rename(path, &to) } else { copy_all(path, &to) };
            done(stream, r, "201 Created")
        }
    }
}

/// Where a MOVE or COPY goes: the `Destination` header's path, under
/// `/files/` or `/dav/`, as a path in the folder.
fn destination(req: &Request, top: &Path) -> Option<PathBuf> {
    let d = req.headers.get("destination")?;
    // An absolute URL loses its scheme and host; a path stays as it is.
    let path = match d.find("://") {
        Some(i) => &d[i + 3..][d[i + 3..].find('/')?..],
        None => d.as_str(),
    };
    let path = decode(path.split('?').next()?);
    let rel = path.strip_prefix("/files/").or_else(|| path.strip_prefix("/dav/"))?;
    let rel = PathBuf::from(rel.trim_end_matches('/'));
    if rel.as_os_str().is_empty() || rel.components().any(|c| !matches!(c, Component::Normal(_))) {
        return None;
    }
    Some(top.join(rel))
}

fn copy_all(from: &Path, to: &Path) -> std::io::Result<()> {
    if from.is_dir() {
        std::fs::create_dir_all(to)?;
        for e in std::fs::read_dir(from)? {
            let e = e?;
            copy_all(&e.path(), &to.join(e.file_name()))?;
        }
        Ok(())
    } else {
        std::fs::copy(from, to).map(|_| ())
    }
}

/// A folder's names as JSON: `[{"name","size","modified","dir"}]`.
fn listing(dir: &Path) -> String {
    let mut json = String::from("[");
    for (i, e) in entries(dir).iter().enumerate() {
        if i > 0 {
            json.push(',');
        }
        json.push_str(&format!(
            "{{\"name\":\"{}\",\"size\":{},\"modified\":{},\"dir\":{}}}",
            json_escape(&e.name),
            e.size,
            e.modified,
            e.dir
        ));
    }
    json.push(']');
    json
}

/// The repository, read-only.
fn statics(stream: &mut TcpStream, req: &Request, file: &Path) -> std::io::Result<()> {
    if req.method != "GET" && req.method != "HEAD" {
        return reply(stream, "405 Method Not Allowed", "text/plain", b"GET only\n", true);
    }
    let mut file = file.to_path_buf();
    // A folder of the repository with ?list: its names, for a page that
    // browses the examples. Still read-only.
    if file.is_dir() && req.query.split('&').any(|q| q == "list") {
        return reply(stream, "200 OK", "application/json", listing(&file).as_bytes(), req.method == "GET");
    }
    if file.is_dir() {
        if !req.path.ends_with('/') {
            // `/Examples/Web` must become `/Examples/Web/`, or the page's
            // relative links resolve one folder too high.
            let head = format!(
                "HTTP/1.1 301 Moved Permanently\r\nLocation: {}/\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                req.path
            );
            return stream.write_all(head.as_bytes());
        }
        file.push("index.html");
    }
    match std::fs::read(&file) {
        Ok(body) => reply(stream, "200 OK", mime(&file), &body, req.method == "GET"),
        Err(_) => reply(stream, "404 Not Found", "text/plain", format!("no {}\n", req.path).as_bytes(), true),
    }
}

/// `/files/`: a folder's names as JSON with `?list`, a file's bytes, a PUT to save.
fn files(stream: &mut TcpStream, req: &Request, path: &Path) -> std::io::Result<()> {
    match req.method.as_str() {
        "GET" | "HEAD" if path.is_dir() => {
            if !req.query.split('&').any(|q| q == "list") {
                return reply(stream, "400 Bad Request", "text/plain", b"a folder: ask with ?list\n", true);
            }
            reply(stream, "200 OK", "application/json", listing(path).as_bytes(), req.method == "GET")
        }
        "GET" | "HEAD" => match std::fs::read(path) {
            Ok(body) => reply(stream, "200 OK", mime(path), &body, req.method == "GET"),
            Err(_) => reply(stream, "404 Not Found", "text/plain", b"no such file\n", true),
        },
        "PUT" => put(stream, req, path),
        _ => reply(stream, "405 Method Not Allowed", "text/plain", b"GET or PUT\n", true),
    }
}

/// `/dav/`: the same folder over WebDAV, as much of it as reading and
/// saving files takes.
fn dav(stream: &mut TcpStream, req: &Request, path: &Path) -> std::io::Result<()> {
    match req.method.as_str() {
        "OPTIONS" => {
            let head = "HTTP/1.1 200 OK\r\nDAV: 1\r\nAllow: OPTIONS, PROPFIND, GET, HEAD, PUT, DELETE, MKCOL, MOVE, COPY\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
            stream.write_all(head.as_bytes())
        }
        "PROPFIND" => {
            let Ok(meta) = std::fs::metadata(path) else {
                return reply(stream, "404 Not Found", "text/plain", b"no such file\n", true);
            };
            let depth = req.headers.get("depth").map(String::as_str).unwrap_or("1");
            let href = if meta.is_dir() && !req.path.ends_with('/') { format!("{}/", req.path) } else { req.path.clone() };
            let mut xml = String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<D:multistatus xmlns:D=\"DAV:\">\n");
            let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            xml.push_str(&dav_response(&href, &Entry::of(&name, &meta)));
            if meta.is_dir() && depth != "0" {
                for e in entries(path) {
                    let child = format!("{}{}{}", href, encode(&e.name), if e.dir { "/" } else { "" });
                    xml.push_str(&dav_response(&child, &e));
                }
            }
            xml.push_str("</D:multistatus>\n");
            reply(stream, "207 Multi-Status", "application/xml; charset=utf-8", xml.as_bytes(), true)
        }
        "GET" | "HEAD" => match std::fs::read(path) {
            Ok(body) => reply(stream, "200 OK", mime(path), &body, req.method == "GET"),
            Err(_) => reply(stream, "404 Not Found", "text/plain", b"no such file\n", true),
        },
        "PUT" => put(stream, req, path),
        _ => reply(stream, "405 Method Not Allowed", "text/plain", b"OPTIONS, PROPFIND, GET or PUT\n", true),
    }
}

/// A file saved: into a folder that exists, never over a folder.
fn put(stream: &mut TcpStream, req: &Request, path: &Path) -> std::io::Result<()> {
    if path.is_dir() {
        return reply(stream, "409 Conflict", "text/plain", b"that is a folder\n", true);
    }
    if !path.parent().is_some_and(|p| p.is_dir()) {
        return reply(stream, "409 Conflict", "text/plain", b"no such folder\n", true);
    }
    let existed = path.exists();
    match std::fs::write(path, &req.body) {
        Ok(()) => reply(stream, if existed { "204 No Content" } else { "201 Created" }, "text/plain", b"", true),
        Err(e) => reply(stream, "500 Internal Server Error", "text/plain", format!("{e}\n").as_bytes(), true),
    }
}

struct Entry {
    name: String,
    size: u64,
    /// Seconds since 1970, UTC.
    modified: u64,
    dir: bool,
}

impl Entry {
    fn of(name: &str, meta: &std::fs::Metadata) -> Entry {
        let modified = meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs());
        Entry { name: name.to_string(), size: if meta.is_dir() { 0 } else { meta.len() }, modified, dir: meta.is_dir() }
    }
}

fn entries(dir: &Path) -> Vec<Entry> {
    let mut v = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            if let Ok(meta) = e.metadata() {
                v.push(Entry::of(&e.file_name().to_string_lossy(), &meta));
            }
        }
    }
    v.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    v
}

fn dav_response(href: &str, e: &Entry) -> String {
    let kind = if e.dir { "<D:collection/>" } else { "" };
    let length = if e.dir { String::new() } else { format!("<D:getcontentlength>{}</D:getcontentlength>", e.size) };
    format!(
        "<D:response><D:href>{}</D:href><D:propstat><D:prop><D:displayname>{}</D:displayname>\
         <D:resourcetype>{kind}</D:resourcetype>{length}<D:getlastmodified>{}</D:getlastmodified>\
         </D:prop><D:status>HTTP/1.1 200 OK</D:status></D:propstat></D:response>\n",
        xml_escape(href),
        xml_escape(&e.name),
        http_date(e.modified)
    )
}

/// Seconds since 1970 as HTTP writes a date: Tue, 30 Sep 2026 12:00:00 GMT.
fn http_date(secs: u64) -> String {
    let days = (secs / 86400) as i64;
    let rest = secs % 86400;
    // Days to a civil date, after Howard Hinnant's algorithm.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + if month <= 2 { 1 } else { 0 };
    const WEEK: [&str; 7] = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"];
    const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    format!(
        "{}, {:02} {} {} {:02}:{:02}:{:02} GMT",
        WEEK[days.rem_euclid(7) as usize],
        day,
        MONTHS[(month - 1) as usize],
        year,
        rest / 3600,
        rest / 60 % 60,
        rest % 60
    )
}

fn json_escape(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 32 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// A name for a URL: letters, digits and a few safe signs as they are,
/// everything else as %XX.
fn encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
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
        "md" | "txt" | "bat" | "cmd" | "asm" | "c" | "h" | "pas" | "rs" => "text/plain; charset=utf-8",
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

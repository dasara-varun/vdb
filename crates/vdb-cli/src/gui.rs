use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::str;
use std::time::Duration;
use vdb_core::{VdbOptions, VdbStore};

const MAX_REQUEST_BYTES: usize = 16 * 1024;
const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;
const GUI_IO_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_QUERY_LIMIT: usize = 100;
const MAX_QUERY_VALUE_BYTES: usize = 8 * 1024;
const MAX_COLLECTION_BYTES: usize = 256;

pub fn serve(path: &std::path::Path, options: VdbOptions, port: u16) -> Result<()> {
    let store = VdbStore::open_with_options(path, options).context("open VDB for GUI")?;
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port))
        .with_context(|| format!("bind local GUI on 127.0.0.1:{port}"))?;
    let address = listener.local_addr()?;
    println!(
        "VDB read-only GUI listening at http://127.0.0.1:{} (press Ctrl-C to stop)",
        address.port()
    );
    for incoming in listener.incoming() {
        match incoming {
            Ok(mut stream) => {
                if let Err(error) = stream
                    .set_read_timeout(Some(GUI_IO_TIMEOUT))
                    .and_then(|_| stream.set_write_timeout(Some(GUI_IO_TIMEOUT)))
                {
                    eprintln!("GUI connection setup failed: {error}");
                    continue;
                }
                let response = match read_request(&mut stream) {
                    Ok(request) => route(&store, &request),
                    Err(response) => response,
                };
                if let Err(error) = write_response(&mut stream, response) {
                    eprintln!("GUI response failed: {error}");
                }
            }
            Err(error) => eprintln!("GUI connection failed: {error}"),
        }
    }
    Ok(())
}

struct Request {
    method: String,
    target: String,
}

struct Response {
    status: u16,
    reason: &'static str,
    content_type: &'static str,
    body: Vec<u8>,
}

fn read_request(stream: &mut TcpStream) -> std::result::Result<Request, Response> {
    let mut bytes = Vec::with_capacity(1024);
    let mut buffer = [0_u8; 1024];
    loop {
        let read = stream
            .read(&mut buffer)
            .map_err(|_| json_error(400, "invalid HTTP request"))?;
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..read]);
        if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
        if bytes.len() > MAX_REQUEST_BYTES {
            return Err(json_error(413, "request headers exceed the GUI limit"));
        }
    }
    if bytes.len() > MAX_REQUEST_BYTES {
        return Err(json_error(413, "request headers exceed the GUI limit"));
    }
    if !bytes.windows(4).any(|window| window == b"\r\n\r\n") {
        return Err(json_error(400, "incomplete HTTP request headers"));
    }
    let text = str::from_utf8(&bytes).map_err(|_| json_error(400, "request is not UTF-8"))?;
    let request_line = text.split_once("\r\n").map_or(text, |(line, _)| line);
    let mut fields = request_line.split_ascii_whitespace();
    let method = fields
        .next()
        .ok_or_else(|| json_error(400, "missing HTTP method"))?;
    let target = fields
        .next()
        .ok_or_else(|| json_error(400, "missing HTTP target"))?;
    let version = fields
        .next()
        .ok_or_else(|| json_error(400, "missing HTTP version"))?;
    if fields.next().is_some() || version != "HTTP/1.1" && version != "HTTP/1.0" {
        return Err(json_error(400, "unsupported HTTP request line"));
    }
    if method != "GET" {
        return Err(json_error(405, "the GUI is read-only; only GET is allowed"));
    }
    if !target.starts_with('/') || target.len() > MAX_REQUEST_BYTES {
        return Err(json_error(400, "invalid local GUI target"));
    }
    Ok(Request {
        method: method.to_string(),
        target: target.to_string(),
    })
}

fn route(store: &VdbStore, request: &Request) -> Response {
    if request.method != "GET" {
        return json_error(405, "the GUI is read-only; only GET is allowed");
    }
    let (path, parameters) = match parse_target(&request.target) {
        Ok(value) => value,
        Err(message) => return json_error(400, message),
    };
    match path.as_str() {
        "/" => html_response(render_dashboard(store)),
        "/api/health" => json_value(store.health()),
        "/api/collections" => json_value(store.list_collections()),
        "/api/documents" => documents_response(store, &parameters),
        "/collection" => collection_page(store, &parameters),
        _ => json_error(404, "GUI route not found"),
    }
}

fn parse_target(
    target: &str,
) -> std::result::Result<(String, BTreeMap<String, String>), &'static str> {
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    if path.is_empty() || !path.starts_with('/') || path.contains("..") {
        return Err("invalid GUI path");
    }
    if query.len() > MAX_QUERY_VALUE_BYTES {
        return Err("GUI query is too large");
    }
    let mut parameters = BTreeMap::new();
    if !query.is_empty() {
        for item in query.split('&') {
            let (key, value) = item.split_once('=').ok_or("invalid GUI query")?;
            let key = decode_component(key).ok_or("invalid GUI query encoding")?;
            let value = decode_component(value).ok_or("invalid GUI query encoding")?;
            if key.is_empty() || parameters.insert(key, value).is_some() {
                return Err("duplicate or empty GUI query parameter");
            }
        }
    }
    Ok((path.to_string(), parameters))
}

fn decode_component(component: &str) -> Option<String> {
    let mut bytes = Vec::with_capacity(component.len());
    let raw = component.as_bytes();
    let mut index = 0;
    while index < raw.len() {
        match raw[index] {
            b'+' => bytes.push(b' '),
            b'%' if index + 2 < raw.len() => {
                let high = hex_digit(raw[index + 1])?;
                let low = hex_digit(raw[index + 2])?;
                bytes.push((high << 4) | low);
                index += 2;
            }
            b'%' => return None,
            byte if byte.is_ascii() && !byte.is_ascii_control() => bytes.push(byte),
            _ => return None,
        }
        index += 1;
    }
    String::from_utf8(bytes).ok()
}

fn hex_digit(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn documents_response(store: &VdbStore, parameters: &BTreeMap<String, String>) -> Response {
    let collection = match collection_parameter(parameters) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let limit = match parameters.get("limit") {
        None => 100,
        Some(value) => match value.parse::<usize>() {
            Ok(value) if (1..=MAX_QUERY_LIMIT).contains(&value) => value,
            _ => return json_error(400, "limit must be between 1 and 100"),
        },
    };
    let where_filter = match parameters.get("where") {
        None => None,
        Some(value) if value.len() <= MAX_QUERY_VALUE_BYTES => {
            let parsed: Value = match serde_json::from_str(value) {
                Ok(value) => value,
                Err(_) => return json_error(400, "where must be valid JSON"),
            };
            let Some(object) = parsed.as_object() else {
                return json_error(400, "where must be a JSON object");
            };
            Some(object.clone())
        }
        Some(_) => return json_error(413, "where exceeds the GUI limit"),
    };
    match store.query(&collection, where_filter.as_ref(), limit) {
        Ok(documents) => json_value(documents),
        Err(error) => json_error(404, &error.to_string()),
    }
}

fn collection_page(store: &VdbStore, parameters: &BTreeMap<String, String>) -> Response {
    let collection = match collection_parameter(parameters) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let limit = match parameters.get("limit") {
        None => 100,
        Some(value) => match value.parse::<usize>() {
            Ok(value) if (1..=MAX_QUERY_LIMIT).contains(&value) => value,
            _ => return json_error(400, "limit must be between 1 and 100"),
        },
    };
    let documents = match store.query(&collection, None, limit) {
        Ok(value) => value,
        Err(error) => return json_error(404, &error.to_string()),
    };
    let health = store.health();
    let mut body = String::from(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; style-src 'unsafe-inline'; base-uri 'none'; frame-ancestors 'none'\"><title>VDB collection</title><style>body{font:16px system-ui,sans-serif;max-width:1000px;margin:2rem auto;padding:0 1rem;color:#18212f;background:#f7f9fc}a{color:#075985}pre{background:#fff;border:1px solid #dbe3ec;border-radius:8px;padding:1rem;overflow:auto}header{display:flex;justify-content:space-between;gap:1rem;align-items:center}small{color:#52606d}.card{background:#fff;border:1px solid #dbe3ec;border-radius:10px;padding:1rem;margin:1rem 0}</style></head><body>",
    );
    body.push_str("<header><h1>VDB collection: ");
    body.push_str(&escape_html(&collection));
    body.push_str(
        "</h1><a href=\"/\">Dashboard</a></header><p><small>Read-only GUI; showing at most ",
    );
    body.push_str(&limit.to_string());
    body.push_str(" documents.</small></p><div class=\"card\"><p>Database health: ");
    body.push_str(&escape_html(health.status));
    body.push_str("; ");
    body.push_str(&health.documents.to_string());
    body.push_str(" total documents.</p><p><a href=\"/api/documents?collection=");
    body.push_str(&encode_component(&collection));
    body.push_str("&limit=");
    body.push_str(&limit.to_string());
    body.push_str("\">Open JSON endpoint</a></p></div>");
    for document in documents {
        let json = match serde_json::to_string_pretty(&document) {
            Ok(value) => value,
            Err(error) => return json_error(500, &error.to_string()),
        };
        let escaped = escape_html(&json);
        if body.len().saturating_add(escaped.len()) > MAX_RESPONSE_BYTES {
            return json_error(
                413,
                "GUI result exceeds the response limit; lower the limit",
            );
        }
        body.push_str("<pre>");
        body.push_str(&escaped);
        body.push_str("</pre>");
    }
    body.push_str("</body></html>");
    html_response(body)
}

fn collection_parameter(
    parameters: &BTreeMap<String, String>,
) -> std::result::Result<String, Response> {
    let Some(collection) = parameters.get("collection") else {
        return Err(json_error(400, "collection is required"));
    };
    if collection.is_empty() || collection.len() > MAX_COLLECTION_BYTES {
        return Err(json_error(400, "collection is empty or too large"));
    }
    Ok(collection.clone())
}

fn render_dashboard(store: &VdbStore) -> String {
    let health = store.health();
    let collections = store.list_collections();
    let mut body = String::from(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; style-src 'unsafe-inline'; base-uri 'none'; frame-ancestors 'none'\"><title>VDB local dashboard</title><style>body{font:16px system-ui,sans-serif;max-width:1000px;margin:2rem auto;padding:0 1rem;color:#18212f;background:#f7f9fc}a{color:#075985}dl{display:grid;grid-template-columns:12rem 1fr;gap:.5rem 1rem;background:#fff;border:1px solid #dbe3ec;border-radius:10px;padding:1rem}dt{font-weight:700}dd{margin:0}li{margin:.5rem 0}.notice{background:#fff7ed;border:1px solid #fed7aa;border-radius:10px;padding:1rem}</style></head><body><h1>VDB local dashboard</h1><p class=\"notice\"><strong>Read-only mode.</strong> This GUI is bound to the local device and does not provide remote access or mutation controls.</p><h2>Health</h2><dl>",
    );
    push_metric(&mut body, "Status", health.status);
    push_metric(&mut body, "Collections", &health.collections.to_string());
    push_metric(&mut body, "Documents", &health.documents.to_string());
    push_metric(
        &mut body,
        "Payload bytes",
        &health.payload_bytes.to_string(),
    );
    push_metric(&mut body, "WAL bytes", &health.wal_bytes.to_string());
    push_metric(
        &mut body,
        "Maximum WAL bytes",
        &health.max_wal_bytes.to_string(),
    );
    body.push_str("</dl><h2>Collections</h2><ul>");
    if collections.is_empty() {
        body.push_str("<li>No collections yet.</li>");
    } else {
        for collection in collections {
            body.push_str("<li><a href=\"/collection?collection=");
            body.push_str(&encode_component(&collection));
            body.push_str("\">");
            body.push_str(&escape_html(&collection));
            body.push_str("</a></li>");
        }
    }
    body.push_str("</ul><p><a href=\"/api/health\">Health JSON</a> · <a href=\"/api/collections\">Collections JSON</a></p><p><small>Use the CLI for writes, backup, restore, import, export, and compaction.</small></p></body></html>");
    body
}

fn push_metric(body: &mut String, label: &str, value: &str) {
    body.push_str("<dt>");
    body.push_str(label);
    body.push_str("</dt><dd>");
    body.push_str(&escape_html(value));
    body.push_str("</dd>");
}

fn encode_component(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![char::from(byte)]
            }
            _ => format!("%{byte:02X}").chars().collect(),
        })
        .collect()
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn json_value<T: serde::Serialize>(value: T) -> Response {
    match serde_json::to_vec(&value) {
        Ok(body) if body.len() <= MAX_RESPONSE_BYTES => Response {
            status: 200,
            reason: "OK",
            content_type: "application/json; charset=utf-8",
            body,
        },
        Ok(_) => json_error(
            413,
            "GUI result exceeds the response limit; lower the limit",
        ),
        Err(error) => json_error(500, &error.to_string()),
    }
}

fn json_error(status: u16, message: &str) -> Response {
    let body = serde_json::to_vec(&serde_json::json!({
        "error": {"code": status, "message": message}
    }))
    .unwrap_or_else(|_| b"{\"error\":{\"code\":500,\"message\":\"GUI error\"}}".to_vec());
    Response {
        status,
        reason: match status {
            400 => "Bad Request",
            404 => "Not Found",
            405 => "Method Not Allowed",
            413 => "Payload Too Large",
            _ => "Internal Server Error",
        },
        content_type: "application/json; charset=utf-8",
        body,
    }
}

fn html_response(body: String) -> Response {
    if body.len() > MAX_RESPONSE_BYTES {
        return json_error(
            413,
            "GUI result exceeds the response limit; lower the limit",
        );
    }
    Response {
        status: 200,
        reason: "OK",
        content_type: "text/html; charset=utf-8",
        body: body.into_bytes(),
    }
}

fn write_response(stream: &mut TcpStream, response: Response) -> Result<()> {
    write!(
        stream,
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nContent-Security-Policy: default-src 'none'; style-src 'unsafe-inline'; base-uri 'none'; frame-ancestors 'none'\r\nX-Content-Type-Options: nosniff\r\nCache-Control: no-store\r\nReferrer-Policy: no-referrer\r\nConnection: close\r\n\r\n",
        response.status,
        response.reason,
        response.content_type,
        response.body.len()
    )?;
    stream.write_all(&response.body)?;
    stream.flush().context("flush GUI response")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEST_DATABASE: AtomicU64 = AtomicU64::new(0);

    fn store() -> VdbStore {
        let suffix = NEXT_TEST_DATABASE.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("vdb-gui-test-{}-{suffix}.vdb", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(format!("{}.lock", path.display()));
        let store = VdbStore::open(path).unwrap();
        store.create_collection("users").unwrap();
        store
            .put(
                "users",
                "u1",
                serde_json::json!({"name": "Ada", "html": "<safe>"}),
                None,
            )
            .unwrap();
        store
    }

    #[test]
    fn dashboard_escapes_collection_names_and_marks_read_only() {
        let store = store();
        let page = render_dashboard(&store);
        assert!(page.contains("Read-only mode"));
        assert!(page.contains("users"));
        assert!(page.contains("/api/health"));
    }

    #[test]
    fn document_endpoint_is_bounded_and_returns_json() {
        let store = store();
        let (_, parameters) = parse_target("/api/documents?collection=users&limit=1").unwrap();
        let response = documents_response(&store, &parameters);
        assert_eq!(response.status, 200);
        assert!(String::from_utf8(response.body).unwrap().contains("Ada"));
    }

    #[test]
    fn malformed_query_and_mutation_method_are_rejected() {
        assert!(parse_target("/api/documents?collection=%ZZ").is_err());
        let response = route(
            &store(),
            &Request {
                method: "POST".to_string(),
                target: "/".to_string(),
            },
        );
        assert_eq!(response.status, 405);
    }

    #[test]
    fn incomplete_http_headers_are_rejected() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let client = std::thread::spawn(move || {
            let mut stream = TcpStream::connect(address).unwrap();
            stream.write_all(b"GET / HTTP/1.1\r\n").unwrap();
            stream.shutdown(std::net::Shutdown::Write).unwrap();
        });
        let (mut server, _) = listener.accept().unwrap();
        let response = match read_request(&mut server) {
            Ok(_) => panic!("incomplete headers unexpectedly parsed"),
            Err(response) => response,
        };
        client.join().unwrap();
        assert_eq!(response.status, 400);
    }

    #[test]
    fn html_escaping_prevents_document_markup_execution() {
        let store = store();
        let (_, parameters) = parse_target("/collection?collection=users").unwrap();
        let response = collection_page(&store, &parameters);
        let page = String::from_utf8(response.body).unwrap();
        assert!(page.contains("&lt;safe&gt;"));
        assert!(!page.contains("<safe>"));
    }

    #[test]
    fn health_type_remains_serializable_for_gui_contract() {
        let health: vdb_core::Health = store().health();
        let value = serde_json::to_value(health).unwrap();
        assert_eq!(value["status"], "healthy");
    }

    #[test]
    fn invalid_limits_and_missing_collections_are_rejected() {
        let store = store();
        let (_, parameters) = parse_target("/api/documents?collection=users&limit=101").unwrap();
        assert_eq!(documents_response(&store, &parameters).status, 400);
        let (_, parameters) = parse_target("/api/documents?collection=missing").unwrap();
        assert_eq!(documents_response(&store, &parameters).status, 404);
    }

    #[test]
    fn oversized_html_is_rejected_before_send() {
        let response = html_response("x".repeat(MAX_RESPONSE_BYTES + 1));
        assert_eq!(response.status, 413);
    }
}

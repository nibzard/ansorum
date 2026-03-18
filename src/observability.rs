use std::sync::LazyLock;
use std::time::Duration;

use chrono::{SecondsFormat, Utc};
use log;
use reqwest::blocking::Client;
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue, USER_AGENT};
use serde::Serialize;
use serde_json::{Value as JsonValue, json};

const EVENT_SCHEMA_VERSION: u8 = 1;
const DEFAULT_HOOK_TIMEOUT_MS: u64 = 2_000;

static EVENT_HOOK: LazyLock<Option<EventHookConfig>> = LazyLock::new(EventHookConfig::from_env);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DispatchMode {
    Sync,
    Async,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EventRecord {
    pub name: &'static str,
    pub payload: JsonValue,
}

#[derive(Clone, Debug)]
struct EventHookConfig {
    url: String,
    timeout_ms: u64,
}

#[derive(Debug, Serialize)]
struct EventEnvelope {
    schema_version: u8,
    emitted_at: String,
    source: EventSource<'static>,
    event: String,
    payload: JsonValue,
}

#[derive(Debug, Serialize)]
struct EventSource<'a> {
    product: &'a str,
    surface: &'a str,
    command: &'a str,
}

impl EventHookConfig {
    fn from_env() -> Option<Self> {
        let url = std::env::var("ANSORUM_EVENT_HOOK_URL").ok()?;
        let url = url.trim();
        if url.is_empty() {
            return None;
        }

        let timeout_ms = std::env::var("ANSORUM_EVENT_HOOK_TIMEOUT_MS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(DEFAULT_HOOK_TIMEOUT_MS);

        Some(Self { url: url.to_string(), timeout_ms })
    }
}

pub fn emit_event(
    surface: &'static str,
    command: &'static str,
    event: impl Into<String>,
    payload: JsonValue,
    dispatch_mode: DispatchMode,
) {
    let envelope = EventEnvelope {
        schema_version: EVENT_SCHEMA_VERSION,
        emitted_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        source: EventSource { product: "ansorum", surface, command },
        event: event.into(),
        payload,
    };

    let serialized = match serde_json::to_string(&envelope) {
        Ok(serialized) => serialized,
        Err(error) => {
            log::warn!("Failed to serialize observability event: {error}");
            return;
        }
    };

    log::info!("{serialized}");

    let Some(config) = EVENT_HOOK.as_ref().cloned() else {
        return;
    };

    match dispatch_mode {
        DispatchMode::Sync => send_event_hook(config, serialized),
        DispatchMode::Async => {
            std::thread::spawn(move || send_event_hook(config, serialized));
        }
    }
}

pub fn machine_delivery_event(
    method: &str,
    request_path: &str,
    served_path: &str,
    content_source: &'static str,
    delivery_mode: &'static str,
) -> Option<EventRecord> {
    let normalized_path = served_path.trim_start_matches('/');

    if normalized_path == "page.md" || normalized_path.ends_with("/page.md") {
        return Some(EventRecord {
            name: "ansorum.markdown.fetch",
            payload: json!({
                "method": method,
                "request_path": request_path,
                "served_path": format!("/{normalized_path}"),
                "content_source": content_source,
                "delivery_mode": delivery_mode,
                "status": 200,
            }),
        });
    }

    let llms_variant = if normalized_path == "llms.txt" {
        Some(("root", None))
    } else if normalized_path == "llms-full.txt" {
        Some(("full", None))
    } else {
        normalized_path
            .strip_suffix("/llms.txt")
            .filter(|pack| !pack.is_empty())
            .map(|pack| ("pack", Some(format!("/{pack}/"))))
    };

    llms_variant.map(|(variant, pack_path)| EventRecord {
        name: "ansorum.llms.fetch",
        payload: json!({
            "method": method,
            "request_path": request_path,
            "served_path": format!("/{normalized_path}"),
            "content_source": content_source,
            "variant": variant,
            "pack_path": pack_path,
            "status": 200,
        }),
    })
}

fn send_event_hook(config: EventHookConfig, body: String) {
    if let Err(error) = send_event_hook_inner(&config, body) {
        log::warn!("Failed to deliver observability event hook to {}: {error}", config.url);
    }
}

fn send_event_hook_inner(config: &EventHookConfig, body: String) -> Result<(), reqwest::Error> {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static(concat!("ansorum/", env!("CARGO_PKG_VERSION"))),
    );

    Client::builder()
        .timeout(Duration::from_millis(config.timeout_ms))
        .default_headers(headers)
        .build()?
        .post(&config.url)
        .body(body)
        .send()?
        .error_for_status()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{EventSource, machine_delivery_event};

    #[test]
    fn classifies_negotiated_markdown_fetches() {
        let event = machine_delivery_event(
            "GET",
            "/refunds/",
            "/refunds/page.md",
            "memory",
            "negotiated",
        )
        .expect("expected markdown event");

        assert_eq!(event.name, "ansorum.markdown.fetch");
        assert_eq!(
            event.payload,
            json!({
                "method": "GET",
                "request_path": "/refunds/",
                "served_path": "/refunds/page.md",
                "content_source": "memory",
                "delivery_mode": "negotiated",
                "status": 200,
            })
        );
    }

    #[test]
    fn classifies_root_and_pack_llms_fetches() {
        let root = machine_delivery_event("GET", "/llms.txt", "/llms.txt", "disk", "direct")
            .expect("expected llms root event");
        assert_eq!(root.name, "ansorum.llms.fetch");
        assert_eq!(
            root.payload,
            json!({
                "method": "GET",
                "request_path": "/llms.txt",
                "served_path": "/llms.txt",
                "content_source": "disk",
                "variant": "root",
                "pack_path": null,
                "status": 200,
            })
        );

        let pack = machine_delivery_event(
            "GET",
            "/billing/llms.txt",
            "/billing/llms.txt",
            "memory",
            "direct",
        )
        .expect("expected llms pack event");
        assert_eq!(
            pack.payload,
            json!({
                "method": "GET",
                "request_path": "/billing/llms.txt",
                "served_path": "/billing/llms.txt",
                "content_source": "memory",
                "variant": "pack",
                "pack_path": "/billing/",
                "status": 200,
            })
        );
    }

    #[test]
    fn source_metadata_stays_stable() {
        let source = EventSource { product: "ansorum", surface: "serve", command: "serve" };
        let value = serde_json::to_value(source).expect("source should serialize");
        assert_eq!(
            value,
            json!({
                "product": "ansorum",
                "surface": "serve",
                "command": "serve",
            })
        );
    }
}

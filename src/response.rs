#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

use std::collections::BTreeMap;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde_json::{Value, json};
#[cfg(target_arch = "wasm32")]
use tma_plugin_api::CatalogItem;
use tma_plugin_api::{MediaStreamResponse, PluginError, PluginHttpRequest, PluginHttpResponse};

use crate::protocol::{VERSION, param, protocol_error};

pub(crate) fn ok_response(
    request: &PluginHttpRequest,
    json_body: Option<Value>,
    xml_body: &str,
) -> PluginHttpResponse {
    let json_mode = param(request, "f").as_deref() == Some("json")
        || request
            .headers
            .get("accept")
            .is_some_and(|value| value.contains("json"));
    let (content_type, body) = if json_mode {
        let body = json!({"subsonic-response": {"status": "ok", "version": VERSION, "type": "tma", "serverVersion": "0.1.0", "openSubsonic": true, "response": json_body.unwrap_or_else(|| json!({}))}});
        (
            "application/json; charset=utf-8",
            serde_json::to_vec(&body).unwrap_or_default(),
        )
    } else {
        let body = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?><subsonic-response status=\"ok\" version=\"{}\" type=\"tma\" serverVersion=\"0.1.0\" openSubsonic=\"true\">{xml_body}</subsonic-response>",
            VERSION
        );
        ("application/xml; charset=utf-8", body.into_bytes())
    };
    PluginHttpResponse {
        status: 200,
        headers: BTreeMap::from([("content-type".into(), content_type.into())]),
        body_b64: Some(BASE64.encode(body)),
    }
}

pub(crate) fn error_response(
    request: &PluginHttpRequest,
    error: PluginError,
) -> PluginHttpResponse {
    let (code, message) = error
        .message
        .strip_prefix("Subsonic error ")
        .and_then(|value| value.split_once(": "))
        .and_then(|(code, message)| code.parse::<u16>().ok().map(|code| (code, message)))
        .unwrap_or((0, error.message.as_str()));
    let json_mode = param(request, "f").as_deref() == Some("json")
        || request
            .headers
            .get("accept")
            .is_some_and(|value| value.contains("json"));
    if json_mode {
        let body = json!({"subsonic-response": {"status": "failed", "version": VERSION, "type": "tma", "serverVersion": "0.1.0", "openSubsonic": true, "error": {"code": code, "message": message}}});
        return PluginHttpResponse {
            status: 200,
            headers: BTreeMap::from([(
                "content-type".into(),
                "application/json; charset=utf-8".into(),
            )]),
            body_b64: Some(BASE64.encode(serde_json::to_vec(&body).unwrap_or_default())),
        };
    }
    let body = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><subsonic-response status=\"failed\" version=\"{}\" type=\"tma\" serverVersion=\"0.1.0\" openSubsonic=\"true\"><error code=\"{}\" message=\"{}\" /></subsonic-response>",
        VERSION,
        code,
        esc(message)
    );
    PluginHttpResponse {
        status: 200,
        headers: BTreeMap::from([(
            "content-type".into(),
            "application/xml; charset=utf-8".into(),
        )]),
        body_b64: Some(BASE64.encode(body.as_bytes())),
    }
}

pub(crate) fn media_response(
    response: MediaStreamResponse,
) -> Result<PluginHttpResponse, PluginError> {
    if !response.ok {
        return Err(protocol_error(
            if response.status == 404 { 70 } else { 0 },
            response
                .error
                .as_ref()
                .map(|error| error.message.as_str())
                .unwrap_or("媒体不可用"),
        ));
    }
    let mut headers = BTreeMap::from([(
        "content-type".into(),
        response
            .content_type
            .unwrap_or_else(|| "application/octet-stream".into()),
    )]);
    if let Some(content_length) = response.content_length {
        headers.insert("content-length".into(), content_length.to_string());
    }
    if let Some(content_range) = response.content_range {
        headers.insert("content-range".into(), content_range);
    }
    if let Some(stream_id) = response.stream_id {
        headers.insert("x-tma-stream-id".into(), stream_id);
    }
    Ok(PluginHttpResponse {
        status: response.status,
        headers,
        body_b64: response.body_b64,
    })
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn required_param(
    request: &PluginHttpRequest,
    name: &str,
) -> Result<String, PluginError> {
    param(request, name)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| protocol_error(10, &format!("缺少参数 {name}")))
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn numeric_param(
    request: &PluginHttpRequest,
    name: &str,
    default: u32,
    max: u32,
) -> Result<u32, PluginError> {
    let Some(value) = param(request, name) else {
        return Ok(default);
    };
    let value = value
        .parse::<u32>()
        .map_err(|_| protocol_error(10, &format!("{name} 必须是整数")))?;
    if value > max {
        return Err(protocol_error(10, &format!("{name} 超出允许范围")));
    }
    Ok(value)
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn song_json(item: &CatalogItem) -> Value {
    json!({"id": item.id, "title": item.title, "album": item.album, "artist": item.artist, "duration": item.duration_ms.map(|v| v / 1000), "track": item.track_number, "discNumber": item.disc_number, "year": item.year, "genre": item.genre, "suffix": item.format, "bitRate": item.bitrate, "coverArt": item.cover_art_id})
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn song_xml(item: &CatalogItem) -> String {
    format!(
        "<song id=\"{}\" title=\"{}\"{}{}{}{} />",
        esc(&item.id),
        esc(&item.title),
        attr_opt("album", item.album.as_deref()),
        attr_opt("artist", item.artist.as_deref()),
        attr_opt("suffix", item.format.as_deref()),
        item.duration_ms
            .map(|value| format!(" duration=\"{}\"", value / 1000))
            .unwrap_or_default()
    )
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn attr_opt(name: &str, value: Option<&str>) -> String {
    value
        .map(|value| format!(" {name}=\"{}\"", esc(value)))
        .unwrap_or_default()
}

pub(crate) fn esc(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    fn request(format: Option<&str>) -> PluginHttpRequest {
        PluginHttpRequest {
            method: "GET".into(),
            path: "/rest/ping.view".into(),
            query: format
                .map(|value| BTreeMap::from([(String::from("f"), vec![value.into()])]))
                .unwrap_or_default(),
            headers: BTreeMap::new(),
            body_b64: None,
            identity: None,
        }
    }

    #[test]
    fn error_response_preserves_structured_subsonic_code_and_escapes_xml() {
        let response = error_response(&request(None), protocol_error(70, "a<b"));
        let body = BASE64.decode(response.body_b64.unwrap()).unwrap();
        let body = String::from_utf8(body).unwrap();
        assert!(body.contains("code=\"70\""));
        assert!(body.contains("a&lt;b"));
    }

    #[test]
    fn json_response_is_subsonic_envelope() {
        let response = ok_response(&request(Some("json")), None, "<ping />");
        let body = BASE64.decode(response.body_b64.unwrap()).unwrap();
        let value: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["subsonic-response"]["status"], "ok");
    }

    #[test]
    fn media_response_forwards_stream_handle_and_length() {
        let response = media_response(MediaStreamResponse {
            version: 1,
            ok: true,
            status: 200,
            content_type: Some("image/jpeg".into()),
            content_length: Some(42),
            content_range: None,
            body_b64: None,
            stream_id: Some("opaque-handle".into()),
            error: None,
        })
        .unwrap();
        assert_eq!(response.headers["x-tma-stream-id"], "opaque-handle");
        assert_eq!(response.headers["content-length"], "42");
    }
}

#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

use tma_plugin_api::{MediaByteRange, PluginError, PluginErrorCode, PluginHttpRequest};

pub(crate) const VERSION: &str = "1.16.1";

// Authentication is deliberately owned by the TMA HTTP host.  The plugin
// accepts `p` only because the host maps it to a scoped plugin credential;
// Subsonic's legacy `t`/`s` pair and `apiKey` are not verifiable here.
const COMMON_PARAMS: &[&str] = &["c", "v", "f", "u", "p"];

pub(crate) fn validate_protocol(request: &PluginHttpRequest) -> Result<(), PluginError> {
    let version = param(request, "v").ok_or_else(|| protocol_error(10, "缺少参数 v"))?;
    if version != VERSION {
        return Err(protocol_error(0, "不支持的 Subsonic 协议版本"));
    }

    let endpoint = endpoint_name(&request.path);
    let endpoint_params: &[&str] = match endpoint {
        "ping" | "getMusicFolders" => &[],
        "getArtists" => &[
            "name",
            "query",
            "musicFolderId",
            "count",
            "offset",
            "sortBy",
            "order",
        ],
        "getAlbum" | "getSong" => &["id"],
        "getCoverArt" => &["id", "size"],
        "stream" => &["id", "format", "maxBitRate", "estimateContentLength"],
        "star" | "unstar" | "setRating" | "scrobble" | "search3" | "getPlaylists" => &[],
        _ => &[],
    };
    for name in request.query.keys() {
        if !COMMON_PARAMS.contains(&name.as_str()) && !endpoint_params.contains(&name.as_str()) {
            return Err(protocol_error(0, &format!("不支持的参数 {name}")));
        }
    }
    Ok(())
}

/// Return the Subsonic endpoint name from a routed `/rest/...` path.
///
/// Subsonic clients conventionally append `.view`; accepting both spellings
/// keeps the HTTP route generic while making the protocol boundary explicit.
pub(crate) fn endpoint_name(path: &str) -> &str {
    let endpoint = path
        .rsplit('/')
        .find(|value| !value.is_empty())
        .unwrap_or("ping");
    endpoint.strip_suffix(".view").unwrap_or(endpoint)
}

/// Parse the one-range form supported by the host media DTO.
pub(crate) fn parse_range(value: &str) -> Result<MediaByteRange, PluginError> {
    let value = value
        .strip_prefix("bytes=")
        .ok_or_else(|| protocol_error(10, "Range 必须使用 bytes= 前缀"))?;
    if value.contains(',') {
        return Err(protocol_error(10, "暂不支持多段 Range"));
    }
    let (start, end) = value
        .trim()
        .split_once('-')
        .ok_or_else(|| protocol_error(10, "Range 格式无效"))?;
    if start.is_empty() {
        return Err(protocol_error(10, "暂不支持后缀 Range"));
    }
    let start = start
        .parse()
        .map_err(|_| protocol_error(10, "Range 起始位置无效"))?;
    let end = (!end.is_empty())
        .then(|| end.parse())
        .transpose()
        .map_err(|_| protocol_error(10, "Range 结束位置无效"))?;
    if end.is_some_and(|end| end < start) {
        return Err(protocol_error(10, "Range 结束位置早于起始位置"));
    }
    Ok(MediaByteRange { start, end })
}

pub(crate) fn param(request: &PluginHttpRequest, name: &str) -> Option<String> {
    request
        .query
        .get(name)
        .and_then(|values| values.first())
        .cloned()
}

pub(crate) fn protocol_error(code: u16, message: &str) -> PluginError {
    PluginError::new(
        PluginErrorCode::PermanentFailure,
        format!("Subsonic error {code}: {message}"),
    )
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    fn request(query: &[(&str, &str)]) -> PluginHttpRequest {
        PluginHttpRequest {
            method: "GET".into(),
            path: "/rest/ping.view".into(),
            query: query
                .iter()
                .map(|(name, value)| ((*name).into(), vec![(*value).into()]))
                .collect::<BTreeMap<_, _>>(),
            headers: BTreeMap::new(),
            body_b64: None,
            identity: None,
        }
    }

    #[test]
    fn accepts_exact_supported_version() {
        assert!(validate_protocol(&request(&[("v", VERSION)])).is_ok());
    }

    #[test]
    fn rejects_missing_or_unknown_version() {
        assert!(validate_protocol(&request(&[])).is_err());
        assert!(validate_protocol(&request(&[("v", "1.16.0")])).is_err());
    }

    #[test]
    fn rejects_unknown_query_parameters() {
        let error = validate_protocol(&request(&[("v", VERSION), ("unexpected", "1")]))
            .expect_err("unknown parameters must be rejected");
        assert!(error.message.contains("unexpected"));
    }

    #[test]
    fn accepts_standard_view_suffix_and_rejects_unsupported_auth_parameters() {
        let mut view_request = request(&[("v", VERSION)]);
        view_request.path = "/rest/getArtists.view".into();
        assert_eq!(endpoint_name(&view_request.path), "getArtists");
        assert!(validate_protocol(&view_request).is_ok());
        for endpoint in ["ping", "getArtists", "stream", "getCoverArt"] {
            assert_eq!(endpoint_name(&format!("/rest/{endpoint}.view")), endpoint);
        }
        for name in ["apiKey", "t", "s"] {
            let request = request(&[("v", VERSION), (name, "value")]);
            assert!(
                validate_protocol(&request).is_err(),
                "{name} must be rejected"
            );
        }
    }

    #[test]
    fn parses_only_single_explicit_byte_ranges() {
        assert_eq!(
            parse_range("bytes=10-20").unwrap(),
            MediaByteRange {
                start: 10,
                end: Some(20)
            }
        );
        assert_eq!(
            parse_range("bytes=10-").unwrap(),
            MediaByteRange {
                start: 10,
                end: None
            }
        );
        assert!(parse_range("bytes=-10").is_err());
        assert!(parse_range("bytes=1-2,4-5").is_err());
        assert!(parse_range("bytes=4-2").is_err());
    }
}

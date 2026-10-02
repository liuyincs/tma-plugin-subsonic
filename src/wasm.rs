#![cfg(target_arch = "wasm32")]

use extism_pdk::{FnResult, Json, host_fn, plugin_fn};
use serde_json::{Value, json};
use tma_plugin_api::{
    CAPABILITY_DTO_VERSION, CatalogReadRequest, CatalogReadResponse, IdentityReadRequest,
    IdentityReadResponse, MediaStreamRequest, MediaStreamResponse, PluginError, PluginErrorCode,
    PluginHttpRequest, PluginHttpResponse,
};

use crate::protocol::{endpoint_name, param, parse_range, protocol_error, validate_protocol};
use crate::response::{
    attr_opt, error_response, esc, media_response, numeric_param, ok_response, required_param,
    song_json, song_xml,
};

const MANIFEST: &str = include_str!("../manifest.json");

#[host_fn("extism:host/user")]
extern "ExtismHost" {
    fn tma_catalog_read(input: Json<CatalogReadRequest>) -> Json<CatalogReadResponse>;
    fn tma_identity_read(input: Json<IdentityReadRequest>) -> Json<IdentityReadResponse>;
    fn tma_media_stream(input: Json<MediaStreamRequest>) -> Json<MediaStreamResponse>;
}

#[plugin_fn]
pub fn tma_manifest() -> FnResult<String> {
    Ok(MANIFEST.to_owned())
}

#[plugin_fn]
pub fn tma_http(input: Json<PluginHttpRequest>) -> FnResult<Json<PluginHttpResponse>> {
    let request = input.0;
    match handle(request.clone()) {
        Ok(response) => Ok(Json(response)),
        Err(error) => Ok(Json(error_response(&request, error))),
    }
}

fn handle(request: PluginHttpRequest) -> Result<PluginHttpResponse, PluginError> {
    validate_protocol(&request)?;
    let endpoint = endpoint_name(&request.path);

    if endpoint != "ping" {
        let identity = identity()?;
        if !identity.ok {
            return Err(protocol_error(50, "未认证"));
        }
    }

    match endpoint {
        "ping" => Ok(ok_response(&request, None, "<ping version=\"1.16.1\" />")),
        "getMusicFolders" => folders(request),
        "getArtists" => artists(request),
        "getAlbum" => album(request),
        "getSong" => song(request),
        "stream" => stream(request),
        "getCoverArt" => cover_art(request),
        "star" | "unstar" | "setRating" | "scrobble" | "search3" | "getPlaylists" => {
            Err(protocol_error(0, "该 Subsonic 端点暂不支持"))
        }
        _ => Err(protocol_error(0, "未知或暂不支持的 Subsonic 端点")),
    }
}

fn identity() -> Result<IdentityReadResponse, PluginError> {
    let response = unsafe {
        tma_identity_read(Json(IdentityReadRequest {
            version: CAPABILITY_DTO_VERSION,
        }))
    }
    .map_err(|error| {
        PluginError::new(
            PluginErrorCode::Internal,
            format!("identity.read 调用失败: {error:?}"),
        )
    })?
    .0;
    Ok(response)
}

fn folders(request: PluginHttpRequest) -> Result<PluginHttpResponse, PluginError> {
    let response = catalog(CatalogReadRequest {
        version: CAPABILITY_DTO_VERSION,
        kind: Some("library".into()),
        limit: 1000,
        ..Default::default()
    })?;
    let items = response.items;
    let json_body = json!({
        "musicFolders": { "musicFolder": items.iter().map(|item| json!({"id": item.id, "name": item.title})).collect::<Vec<_>>() }
    });
    let xml = format!(
        "<musicFolders>{}</musicFolders>",
        items
            .iter()
            .map(|item| format!(
                "<musicFolder id=\"{}\" name=\"{}\" />",
                esc(&item.id),
                esc(&item.title)
            ))
            .collect::<String>()
    );
    Ok(ok_response(&request, Some(json_body), &xml))
}

fn artists(request: PluginHttpRequest) -> Result<PluginHttpResponse, PluginError> {
    let count = numeric_param(&request, "count", 20, 500)?;
    let offset = numeric_param(&request, "offset", 0, u32::MAX)?;
    if param(&request, "musicFolderId").is_some() {
        return Err(protocol_error(0, "暂不支持 musicFolderId 过滤"));
    }
    if param(&request, "sortBy").is_some_and(|value| value != "name")
        || param(&request, "order").is_some_and(|value| value != "asc")
    {
        return Err(protocol_error(0, "仅支持按名称升序获取艺术家"));
    }
    let response = catalog(CatalogReadRequest {
        version: CAPABILITY_DTO_VERSION,
        kind: Some("artist".into()),
        query: param(&request, "name").or_else(|| param(&request, "query")),
        cursor: (offset > 0).then(|| offset.to_string()),
        limit: count,
        ..Default::default()
    })?;
    let json_artists: Vec<Value> = response
        .items
        .iter()
        .map(|item| json!({"id": item.id, "name": item.title}))
        .collect();
    let json_body = json!({"artists": {"index": [{"name": "", "artist": json_artists}]}});
    let xml = format!(
        "<artists><index name=\"\">{}</index></artists>",
        response
            .items
            .iter()
            .map(|item| format!(
                "<artist id=\"{}\" name=\"{}\" />",
                esc(&item.id),
                esc(&item.title)
            ))
            .collect::<String>()
    );
    Ok(ok_response(&request, Some(json_body), &xml))
}

fn album(request: PluginHttpRequest) -> Result<PluginHttpResponse, PluginError> {
    let id = required_param(&request, "id")?;
    let response = catalog(CatalogReadRequest {
        version: CAPABILITY_DTO_VERSION,
        kind: Some("album".into()),
        id: Some(id.clone()),
        limit: 1,
        ..Default::default()
    })?;
    let Some(item) = response.items.first() else {
        return Err(protocol_error(70, "专辑不存在"));
    };
    let tracks = catalog(CatalogReadRequest {
        version: CAPABILITY_DTO_VERSION,
        kind: Some("song".into()),
        parent_id: Some(id.clone()),
        limit: 1000,
        ..Default::default()
    })?;
    let json_body = json!({"album": {"id": item.id, "name": item.title, "artist": item.artist, "year": item.year, "song": tracks.items.iter().map(song_json).collect::<Vec<_>>()}});
    let xml = format!(
        "<album id=\"{}\" name=\"{}\"{}>{}</album>",
        esc(&item.id),
        esc(&item.title),
        attr_opt("artist", item.artist.as_deref()),
        tracks.items.iter().map(song_xml).collect::<String>()
    );
    Ok(ok_response(&request, Some(json_body), &xml))
}

fn song(request: PluginHttpRequest) -> Result<PluginHttpResponse, PluginError> {
    let id = required_param(&request, "id")?;
    let response = catalog(CatalogReadRequest {
        version: CAPABILITY_DTO_VERSION,
        kind: Some("song".into()),
        id: Some(id),
        limit: 1,
        ..Default::default()
    })?;
    let Some(item) = response.items.first() else {
        return Err(protocol_error(70, "歌曲不存在"));
    };
    let json_body = json!({"song": song_json(item)});
    let xml = song_xml(item);
    Ok(ok_response(&request, Some(json_body), &xml))
}

fn stream(request: PluginHttpRequest) -> Result<PluginHttpResponse, PluginError> {
    let id = required_param(&request, "id")?;
    let range = request
        .headers
        .get("range")
        .map(|value| parse_range(value))
        .transpose()?;
    let codec = param(&request, "format");
    let bitrate_kbps = param(&request, "maxBitRate")
        .map(|value| {
            value
                .parse()
                .map_err(|_| protocol_error(10, "maxBitRate 必须是整数"))
        })
        .transpose()?;
    let response = unsafe {
        tma_media_stream(Json(MediaStreamRequest {
            version: CAPABILITY_DTO_VERSION,
            media_id: id,
            codec,
            bitrate_kbps,
            range,
            client_capabilities: None,
        }))
    }
    .map_err(|error| {
        PluginError::new(
            PluginErrorCode::Internal,
            format!("media.stream 调用失败: {error:?}"),
        )
    })?
    .0;
    media_response(response)
}

fn cover_art(request: PluginHttpRequest) -> Result<PluginHttpResponse, PluginError> {
    let id = required_param(&request, "id")?;
    if param(&request, "size").is_some() {
        return Err(protocol_error(0, "暂不支持按 size 缩放封面"));
    }
    let media_id =
        if id.starts_with("cover:") || id.starts_with("track:") || id.starts_with("artist:") {
            id
        } else {
            format!("cover:{id}")
        };
    let response = unsafe {
        tma_media_stream(Json(MediaStreamRequest {
            version: CAPABILITY_DTO_VERSION,
            media_id,
            codec: None,
            bitrate_kbps: None,
            range: None,
            client_capabilities: None,
        }))
    }
    .map_err(|error| {
        PluginError::new(
            PluginErrorCode::Internal,
            format!("media.stream 调用失败: {error:?}"),
        )
    })?
    .0;
    media_response(response)
}

fn catalog(request: CatalogReadRequest) -> Result<CatalogReadResponse, PluginError> {
    let response = unsafe { tma_catalog_read(Json(request)) }
        .map_err(|error| {
            PluginError::new(
                PluginErrorCode::Internal,
                format!("catalog.read 调用失败: {error:?}"),
            )
        })?
        .0;
    if !response.ok {
        return Err(protocol_error(
            0,
            response
                .error
                .as_ref()
                .map(|e| e.message.as_str())
                .unwrap_or("catalog 读取失败"),
        ));
    }
    Ok(response)
}

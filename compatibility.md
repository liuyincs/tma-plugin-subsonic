# Compatibility

| Item | Value |
| --- | --- |
| Plugin ID | `tma.community.subsonic` |
| Plugin release | `0.1.0` |
| Host ABI | `1.6` |
| Subsonic protocol | `1.16.1` response envelope |
| Authentication | TMA plugin credential through `Bearer`, `credential`, or Subsonic `p=enc:<hex>`; `apiKey` and legacy `t`/`s` are rejected |
| Response formats | XML by default; JSON with `f=json` or an `Accept` header containing `json` |
| Streaming | Original bytes or TMA transcode profiles (`format`/`maxBitRate`) with HTTP Range support |
| Mutations | None |

The public plugin repository is buildable only after TMA publishes an immutable
revision containing the ABI 1.6 contract. The release workflow requires that
revision through `TMA_PLUGIN_DEV_REV` and fails closed while it is unavailable.

Requests must declare Subsonic version `1.16.1`; unknown query parameters and
unknown endpoints return the standard failed response. `getArtists` honors the
standard `count` and `offset` paging parameters within the host catalog limit.

Implemented endpoints are `ping`, `getMusicFolders`, `getArtists`, `getAlbum`,
`getSong`, `getCoverArt`, and `stream` (both `/rest/name` and the standard
`/rest/name.view` spelling). Unsupported write, search, scrobble,
and playlist methods return a standard Subsonic failed response.

The plugin does not implement Subsonic's legacy `t`/`s` MD5 credential pair;
TMA stores only one-way plugin credential verifiers. Transcoding is delegated to
the host's existing format registry and cache; the plugin never reads files or
invokes ffmpeg directly.

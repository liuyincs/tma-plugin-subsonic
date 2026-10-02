# Tag My Audio Subsonic plugin

This repository contains the read-only Subsonic / OpenSubsonic adapter for
[Tag My Audio](https://github.com/liuyincs/tma). It runs in TMA's WASM plugin
sandbox and exposes `/rest/*` without adding Subsonic protocol code to the
server core.

## Install

Install the signed `.tmap` package from the static URL in [`catalog.json`](catalog.json)
through TMA's administrator plugin page. The host must support ABI 1.6 and
explicitly trust the public key in [`public-key.txt`](public-key.txt).

A TMA user then creates a credential in Profile → External clients. Send that
credential as `Authorization: Bearer <secret>`, `credential=<secret>`, or the
Subsonic-compatible `p=enc:<hex-secret>` parameter. Subsonic `apiKey` and the
legacy `t`/`s` pair are rejected because the host cannot verify them against a
scoped TMA plugin credential.

## Supported surface

The first release supports `ping`, `getMusicFolders`, `getArtists`, `getAlbum`,
`getSong`, `getCoverArt`, and `stream`. Responses are XML by default and JSON
when the client asks for `f=json` or an `Accept` header containing `json`.
Playback accepts the Subsonic `format` and `maxBitRate` parameters and delegates
transcoding, caching, and byte ranges to the host. See
[`compatibility.md`](compatibility.md) for the user-visible boundaries.

## Build locally

The package targets `wasm32-unknown-unknown`:

```sh
cargo build --target wasm32-unknown-unknown --release
```

The `tma-plugin-api` dependency must resolve to a TMA host release that
contains the ABI 1.6 HTTP and capability DTOs. The public TMA repository does
not yet contain that release, so `Cargo.toml` intentionally records the
unreleased branch and this repository is not release-ready until an immutable
ABI 1.6 revision can be pinned. Release signing is performed by GitHub Actions;
the private signing key is never checked into this repository.

use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
struct Manifest {
    id: String,
    version: String,
    abi: Abi,
    extension_points: Vec<String>,
    permissions: Vec<Permission>,
    http: Http,
}

#[derive(Debug, Deserialize)]
struct Abi {
    min: Version,
    max: Version,
}

#[derive(Debug, Deserialize)]
struct Version {
    major: u16,
    minor: u16,
}

#[derive(Debug, Deserialize)]
struct Permission {
    capability: Capability,
}

#[derive(Debug, Deserialize)]
struct Capability {
    name: String,
    reason: String,
}

#[derive(Debug, Deserialize)]
struct Http {
    routes: Vec<Route>,
}

#[derive(Debug, Deserialize)]
struct Route {
    path: String,
    methods: Vec<String>,
}

#[test]
fn manifest_declares_stable_subsonic_contract() {
    let raw = include_str!("../manifest.json");
    let manifest: Manifest = serde_json::from_str(raw).expect("manifest.json must be valid JSON");
    assert_eq!(manifest.id, "tma.community.subsonic");
    assert_eq!(manifest.version, "0.1.0");
    assert_eq!((manifest.abi.min.major, manifest.abi.min.minor), (1, 6));
    assert_eq!((manifest.abi.max.major, manifest.abi.max.minor), (1, 6));
    assert_eq!(manifest.extension_points, vec!["http"]);
    assert_eq!(
        manifest
            .permissions
            .iter()
            .map(|permission| permission.capability.name.as_str())
            .collect::<Vec<_>>(),
        vec!["catalog.read", "identity.read", "media.stream"]
    );
    assert!(
        manifest.permissions.iter().all(|permission| !permission
            .capability
            .reason
            .trim()
            .is_empty())
    );
    assert_eq!(manifest.http.routes.len(), 1);
    assert_eq!(manifest.http.routes[0].path, "/rest/*");
    assert_eq!(
        manifest.http.routes[0].methods,
        vec!["GET".to_string(), "HEAD".to_string()]
    );
}

#[test]
fn catalog_is_compatible_with_tma_repo_parser() {
    let raw = include_str!("../catalog.json");
    let catalog: Value = serde_json::from_str(raw).expect("catalog.json must be valid JSON");
    let entry = &catalog["plugins"][0];
    assert_eq!(entry["id"], "tma.community.subsonic");
    assert_eq!(entry["version"], "0.1.0");
    assert!(
        entry["download_url"]
            .as_str()
            .unwrap()
            .ends_with("subsonic.tmap")
    );
    let sha = entry["sha256"].as_str().unwrap();
    assert_eq!(sha.len(), 64);
    assert!(sha.chars().all(|ch| ch.is_ascii_hexdigit()));
}

// where: iclaw/canister/src/http/assets.rs
// what: Embedded asset definitions and certification config for the caller UI
// why: Keep the HTTP serving logic small and move per-asset wiring into one module

use ic_asset_certification::{Asset, AssetConfig, AssetFallbackConfig, AssetRouter};
use ic_http_certification::StatusCode;

pub struct EmbeddedUiAsset {
    pub path: &'static str,
    pub bytes: &'static [u8],
    pub content_type: &'static str,
    pub cache_control: &'static str,
}

include!(concat!(env!("OUT_DIR"), "/ui_assets.rs"));

pub fn build_asset_router() -> AssetRouter<'static> {
    let mut router = AssetRouter::default();
    let assets = UI_ASSETS
        .iter()
        .map(|asset| Asset::new(asset.path, asset.bytes))
        .collect::<Vec<_>>();
    let configs = UI_ASSETS.iter().map(asset_config).collect::<Vec<_>>();
    router
        .certify_assets(assets, configs)
        .expect("embedded ui assets should certify");
    router
}

fn asset_config(asset: &EmbeddedUiAsset) -> AssetConfig {
    let headers = vec![
        ("cache-control".to_string(), asset.cache_control.to_string()),
        ("content-type".to_string(), asset.content_type.to_string()),
        ("x-content-type-options".to_string(), "nosniff".to_string()),
        (
            "content-security-policy".to_string(),
            "default-src 'self'; img-src 'self' data: https://identity.internetcomputer.org; style-src 'self' 'unsafe-inline'; script-src 'self'; connect-src 'self' https://identity.internetcomputer.org https://icp0.io https://*.icp0.io https://ic0.app https://*.ic0.app http://127.0.0.1:* http://localhost:* http://*.localhost:*; frame-src https://identity.internetcomputer.org http://*.localhost:*; base-uri 'none'; form-action 'self'; object-src 'none'"
                .to_string(),
        ),
        ("referrer-policy".to_string(), "same-origin".to_string()),
    ];

    if asset.path == UI_INDEX_PATH {
        AssetConfig::File {
            path: asset.path.to_string(),
            content_type: Some(asset.content_type.to_string()),
            headers,
            fallback_for: vec![AssetFallbackConfig {
                scope: "/".to_string(),
                status_code: Some(StatusCode::OK),
            }],
            aliased_by: vec!["/".to_string()],
            encodings: vec![],
        }
    } else {
        AssetConfig::File {
            path: asset.path.to_string(),
            content_type: Some(asset.content_type.to_string()),
            headers,
            fallback_for: vec![],
            aliased_by: vec![format!("/{}", asset.path)],
            encodings: vec![],
        }
    }
}

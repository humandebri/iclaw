// where: standalone/canister/src/http/mod.rs
// what: Single-canister HTTP asset serving for the embedded caller UI
// why: iclaw_ic now ships its own operational frontend and must serve certified static assets itself

mod assets;

use self::assets::build_asset_router;
use ic_asset_certification::AssetRouter;
use ic_http_certification::{HttpRequest, HttpResponse, Method, StatusCode};
use std::cell::RefCell;

#[cfg(test)]
use self::assets::UI_ASSETS;

thread_local! {
    static ROUTER: RefCell<AssetRouter<'static>> = RefCell::new(build_asset_router());
}

pub fn init_assets() {
    ROUTER.with(|router_cell| {
        let next_router = build_asset_router();
        maybe_set_certified_data(&next_router.root_hash());
        *router_cell.borrow_mut() = next_router;
    });
}

pub fn serve(request: HttpRequest<'static>) -> HttpResponse<'static> {
    if request.method() != Method::GET && request.method() != Method::HEAD {
        return HttpResponse::builder()
            .with_status_code(StatusCode::METHOD_NOT_ALLOWED)
            .with_headers(vec![
                (
                    "content-type".to_string(),
                    "text/plain; charset=utf-8".to_string(),
                ),
                ("allow".to_string(), "GET, HEAD".to_string()),
            ])
            .with_body(b"method not allowed".to_vec())
            .build();
    }

    ROUTER.with(|router| {
        let certificate = current_data_certificate();
        match router.borrow().serve_asset(&certificate, &request) {
            Ok(response) => {
                if request.method() == Method::HEAD {
                    HttpResponse::builder()
                        .with_status_code(response.status_code())
                        .with_headers(response.headers().to_vec())
                        .with_body(Vec::<u8>::new())
                        .with_upgrade(response.upgrade().unwrap_or(false))
                        .build()
                } else {
                    response
                }
            }
            Err(_) => HttpResponse::not_found(
                b"asset not found".to_vec(),
                vec![(
                    "content-type".to_string(),
                    "text/plain; charset=utf-8".to_string(),
                )],
            )
            .build(),
        }
    })
}

#[cfg(target_arch = "wasm32")]
fn current_data_certificate() -> Vec<u8> {
    ic_cdk::api::data_certificate().unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
fn current_data_certificate() -> Vec<u8> {
    vec![1, 2, 3]
}

#[cfg(target_arch = "wasm32")]
fn maybe_set_certified_data(root_hash: &[u8]) {
    ic_cdk::api::certified_data_set(root_hash);
}

#[cfg(not(target_arch = "wasm32"))]
fn maybe_set_certified_data(_root_hash: &[u8]) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_request_returns_html() {
        let response = serve(HttpRequest::get("/").build());
        assert_eq!(response.status_code(), StatusCode::OK);
        assert!(response
            .headers()
            .iter()
            .any(|(name, value)| name.eq_ignore_ascii_case("content-type")
                && value.starts_with("text/html")));
    }

    #[test]
    fn unknown_path_falls_back_to_index() {
        let response = serve(HttpRequest::get("/missing/path").build());
        assert_eq!(response.status_code(), StatusCode::OK);
        assert!(String::from_utf8_lossy(response.body()).contains("iclaw IC Console"));
    }

    #[test]
    fn asset_request_uses_certified_headers() {
        let asset = UI_ASSETS
            .iter()
            .find(|entry| entry.path.ends_with(".js"))
            .expect("js asset should exist");
        let response = serve(HttpRequest::get(format!("/{}", asset.path)).build());
        assert_eq!(response.status_code(), StatusCode::OK);
        assert!(response
            .headers()
            .iter()
            .any(|(name, _)| name == "IC-Certificate"));
    }
}

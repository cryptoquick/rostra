mod add_followee;
mod new_post;
mod self_account;
mod timeline;
mod unlock;

use std::sync::Arc;

use axum::body::Body;
use axum::extract::{FromRequest, Path, Request, State};
use axum::http::header::{self, ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_TYPE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::Router;
use maud::Markup;

use super::error::{RequestError, UserErrorResponse};
use super::SharedState;
use crate::asset_cache::AssetCache;

#[derive(Clone, Debug)]
#[must_use]
pub struct Maud(pub Markup);

impl IntoResponse for Maud {
    fn into_response(self) -> Response {
        (
            [(
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/html; charset=utf-8"),
            )],
            self.0 .0,
        )
            .into_response()
    }
}

#[derive(FromRequest)]
#[from_request(via(axum::Json), rejection(RequestError))]
pub struct AppJson<T>(pub T);

impl<T> IntoResponse for AppJson<T>
where
    axum::Json<T>: IntoResponse,
{
    fn into_response(self) -> Response {
        axum::Json(self.0).into_response()
    }
}

pub fn static_file_handler(assets: AssetCache) -> Router {
    let assets = Arc::new(assets);
    Router::new().route(
        "/{*file}",
        get(
            // |state: State<SharedState>, path: Path<String>, req_headers: HeaderMap| async move {
            |path: Path<String>, req_headers: HeaderMap| async move {
                let Some(asset) = assets.get_from_path(&path) else {
                    return StatusCode::NOT_FOUND.into_response();
                };

                let mut resp_headers = HeaderMap::new();

                // We set the content type explicitly here as it will otherwise
                // be inferred as an `octet-stream`
                resp_headers.insert(
                    CONTENT_TYPE,
                    HeaderValue::from_static(
                        asset.content_type().unwrap_or("application/octet-stream"),
                    ),
                );

                let accepts_brotli =
                    req_headers
                        .get_all(ACCEPT_ENCODING)
                        .into_iter()
                        .any(|encodings| {
                            let Ok(str) = encodings.to_str() else {
                                return false;
                            };

                            str.split(',').any(|s| s.trim() == "br")
                        });

                let content = match (accepts_brotli, asset.compressed.as_ref()) {
                    (true, Some(compressed)) => {
                        resp_headers.insert(CONTENT_ENCODING, HeaderValue::from_static("br"));

                        compressed.clone()
                    }
                    _ => asset.raw.clone(),
                };

                (resp_headers, content).into_response()
            },
        ),
    )
}

pub async fn cache_control(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;

    if let Some(content_type) = response.headers().get(CONTENT_TYPE) {
        const NON_CACHEABLE_CONTENT_TYPES: &[&str] = &["text/html"];

        if NON_CACHEABLE_CONTENT_TYPES
            .iter()
            .all(|&ct| !content_type.as_bytes().starts_with(ct.as_bytes()))
        {
            let value = format!("public, max-age={}", 24 * 60 * 60);

            if let Ok(value) = HeaderValue::from_str(&value) {
                response.headers_mut().insert("cache-control", value);
            }
        }
    }

    response
}

pub async fn not_found(_state: State<SharedState>, _req: Request<Body>) -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        AppJson(UserErrorResponse {
            message: "Not Found".to_string(),
        }),
    )
}

pub fn route_handler(state: SharedState) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/ui", get(timeline::get))
        .route("/ui/post", post(new_post::post_new_post))
        .route("/ui/post/preview", post(new_post::get_post_preview))
        .route("/ui/followee", post(add_followee::add_followee))
        .route("/ui/unlock", get(unlock::get).post(unlock::post))
        .route("/ui/unlock/logout", get(unlock::get).post(unlock::logout))
        .route("/ui/unlock/random", get(unlock::get_random))
        .route("/ui/timeline/updates", get(timeline::get_updates))
        .route(
            "/ui/self/edit",
            get(self_account::get_self_account_edit).post(self_account::post_self_account_edit),
        )
        // .route("/a/", put(account_new))
        // .route("/t/", put(token_new))
        // .route("/m/", put(metric_new).get(metric_find))
        // .route("/m/:metric", post(metric_post).get(metric_get_default_type))
        // .route("/m/:metric/:type", get(metric_get))
        .fallback(not_found)
        .with_state(state)
        .layer(middleware::from_fn(cache_control))
}

async fn root() -> Redirect {
    Redirect::permanent("/ui")
}

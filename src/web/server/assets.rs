//! Static asset serving.
//!
//! The frontend is split into per-page CSS and ES-module JS files. They are
//! embedded at compile time and served through one handler keyed by path, so
//! adding a component/page only means adding a line to `asset()`.

use axum::{
    extract::Path as AxumPath,
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
};

pub(super) async fn index() -> Html<&'static str> {
    Html(include_str!("../assets/index.html"))
}

const CSS: &str = "text/css; charset=utf-8";
const JS: &str = "application/javascript; charset=utf-8";

/// Map an `/assets/<path>` request to its embedded (content-type, body).
fn asset(path: &str) -> Option<(&'static str, &'static str)> {
    Some(match path {
        // stylesheets — one global base plus one file per page (non-modular)
        "css/base.css" => (CSS, include_str!("../assets/css/base.css")),
        "css/dashboard.css" => (CSS, include_str!("../assets/css/dashboard.css")),
        "css/system.css" => (CSS, include_str!("../assets/css/system.css")),
        "css/cleaner.css" => (CSS, include_str!("../assets/css/cleaner.css")),
        // third-party (vendored so the app stays self-contained offline)
        "js/vendor/chart.umd.js" => (JS, include_str!("../assets/js/vendor/chart.umd.js")),
        // app shell + shared libs
        "js/app.js" => (JS, include_str!("../assets/js/app.js")),
        "js/lib/api.js" => (JS, include_str!("../assets/js/lib/api.js")),
        "js/lib/format.js" => (JS, include_str!("../assets/js/lib/format.js")),
        "js/lib/metrics.js" => (JS, include_str!("../assets/js/lib/metrics.js")),
        // components
        "js/components/sidebar.js" => (JS, include_str!("../assets/js/components/sidebar.js")),
        // pages
        "js/pages/dashboard.js" => (JS, include_str!("../assets/js/pages/dashboard.js")),
        "js/pages/system.js" => (JS, include_str!("../assets/js/pages/system.js")),
        "js/pages/cleaner.js" => (JS, include_str!("../assets/js/pages/cleaner.js")),
        _ => return None,
    })
}

pub(super) async fn serve_asset(AxumPath(path): AxumPath<String>) -> Response {
    match asset(&path) {
        Some((content_type, body)) => {
            ([(header::CONTENT_TYPE, content_type)], body).into_response()
        }
        None => (StatusCode::NOT_FOUND, "Not found").into_response(),
    }
}

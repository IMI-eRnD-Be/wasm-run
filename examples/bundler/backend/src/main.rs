use rust_embed::RustEmbed;
use warp::{filters::BoxedFilter, http::header::HeaderValue, path::Tail, reply::Response, Filter};

#[tokio::main]
async fn main() {
    let routes = warp::path!("api" / ..)
        .and(warp::path::tail())
        .map(|_| "Please pretend there's an API here.")
        .or(ui_routes());

    warp::serve(routes).bind(([127, 0, 0, 1], 3030)).await;
}

#[derive(RustEmbed)]
#[folder = "$OUT_DIR/ui"]
struct PortalWebAssets;

pub fn ui_routes() -> BoxedFilter<(impl warp::Reply,)> {
    let static_files = warp::get()
        .and(warp::path::tail())
        .and_then(|path: Tail| async move { serve(path.as_str()).await })
        .boxed();

    let spa_mode_index = warp::get()
        .and(warp::path::tail())
        .and_then(|path: Tail| async move {
            let first_path_segment = path.as_str().split('/').next();
            let last_path_segment = path.as_str().split('/').last();
            let is_api_path = first_path_segment == Some("api");
            let is_index_html = last_path_segment == Some("index.html");
            let is_file_like_path = last_path_segment
                .map(|segment| segment.contains('.'))
                .unwrap_or(false);

            if !is_api_path && (is_index_html || !is_file_like_path) {
                serve("index.html").await
            } else {
                Err(warp::reject::not_found())
            }
        })
        .boxed();

    // Order is important here. Serve a file if it exists, then fall back to index.html as a default.
    static_files.or(spa_mode_index).boxed()
}

async fn serve(path: &str) -> Result<impl warp::Reply, warp::Rejection> {
    if let Some(asset) = PortalWebAssets::get(path) {
        let mime = mime_guess::from_path(path).first_or_octet_stream();

        let mut res = Response::new(asset.into());
        res.headers_mut().insert(
            "content-type",
            HeaderValue::from_str(mime.as_ref()).unwrap(),
        );
        Ok(res)
    } else {
        Err(warp::reject::not_found())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use warp::http::status::StatusCode;

    #[tokio::test]
    async fn test_html() {
        let filter = ui_routes();
        let response = warp::test::request().path("/").reply(&filter).await;

        assert_eq!(response.status(), StatusCode::OK);

        let response_body: String = String::from_utf8(response.body().to_vec()).unwrap();

        assert!(response_body
            .contains("<style>button{background:black;color:white;font-size:100px}\n</style>"));
        assert!(response_body.contains("<title>Seed Quickstart</title>"));

        assert!(response_body.contains("<script type=\"module\">"));
    }

    #[tokio::test]
    async fn test_wasm() {
        let filter = ui_routes();
        let response = warp::test::request()
            .path("/app-0.0.0.wasm")
            .reply(&filter)
            .await;

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_static_file() {
        let filter = ui_routes();
        let response = warp::test::request()
            .path("/static/star.png")
            .reply(&filter)
            .await;

        assert_eq!(response.status(), StatusCode::OK);
    }
}

use crate::ServeCmd;
use anyhow::Result;
use axum::{routing::get, Router};

pub async fn serve(args: ServeCmd) -> Result<()> {
    let app = Router::new().route("/", get(|| async { "Hello, World!" }));

    Ok(axum::Server::bind(&args.bind_to.parse()?)
        .serve(app.into_make_service())
        .await?)
}

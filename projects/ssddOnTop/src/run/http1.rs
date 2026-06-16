use crate::app_ctx::AppCtx;
use crate::http::request_handler::handle_request;
use hyper::service::service_fn;
use tokio::net::TcpListener;

pub async fn start(app_ctx: AppCtx) -> anyhow::Result<()> {
    let addr = app_ctx.blueprint.server.addr();
    let listener = TcpListener::bind(addr).await?;

    tracing::info!("Listening on: http://{}", addr);
    loop {
        let app_ctx = app_ctx.clone();

        let stream_result = listener.accept().await;
        match stream_result {
            Ok((stream, _)) => {
                let io = hyper_util::rt::TokioIo::new(stream);
                tokio::spawn(async move {
                    let app_ctx = app_ctx.clone();

                    let server = hyper::server::conn::http1::Builder::new()
                        .serve_connection(
                            io,
                            service_fn(move |req| {
                                let app_ctx = app_ctx.clone();

                                async move {
                                    let req =
                                        crate::http::request::Request::from_hyper(req).await?;
                                    handle_request(req, app_ctx).await
                                }
                            }),
                        )
                        .await;
                    if let Err(e) = server {
                        tracing::error!("An error occurred while handling a request: {e}");
                    }
                });
            }
            Err(e) => tracing::error!("An error occurred while handling request: {e}"),
        }
    }
}

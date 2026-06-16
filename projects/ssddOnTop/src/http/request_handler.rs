use crate::app_ctx::AppCtx;
use crate::blueprint::model::{FieldName, TypeName};
use crate::blueprint::FieldHash;
use crate::http::method::Method;
use crate::http::request::Request;
use crate::ir::eval_ctx::EvalContext;
use crate::request_context::RequestContext;
use crate::value::Value;
use bytes::Bytes;
use http_body_util::Full;
use crate::jit::model::PathFinder;

pub async fn handle_request(
    req: Request,
    app_ctx: AppCtx,
) -> anyhow::Result<hyper::Response<Full<Bytes>>> {
    let resp = match req.method {
        Method::GET => hyper::Response::builder()
            .status(hyper::StatusCode::OK)
            .body(Full::new(Bytes::from(async_graphql::http::GraphiQLSource::build().finish())))?,
        Method::POST => handle_gql_req(req, app_ctx).await?,
        _ => hyper::Response::builder()
            .status(hyper::StatusCode::METHOD_NOT_ALLOWED)
            .body(Full::new(Bytes::from_static(b"Method Not Allowed")))?,
    };
    Ok(resp)
}

fn create_request_context(app_ctx: &AppCtx) -> RequestContext {
    RequestContext::from(app_ctx)
}

async fn handle_gql_req(
    request: Request,
    app_ctx: AppCtx,
) -> anyhow::Result<hyper::Response<Full<Bytes>>> {
    let gql_req: async_graphql::Request = serde_json::from_slice(&request.body)?;
    let doc = async_graphql::parser::parse_query(&gql_req.query)?;
    let req_ctx = create_request_context(&app_ctx);
    if let Some(_) = app_ctx.blueprint.schema.query.as_ref() {
        let eval_ctx = EvalContext::new(&req_ctx);
        let path_finder = PathFinder::new(doc, &app_ctx.blueprint);
        let fields = path_finder.exec().await;
        let borrowed_fields = fields.to_borrowed();

        let resolved = fields.resolve(eval_ctx).await?;
        let borrowed_val = resolved.to_borrowed();
        Ok(hyper::Response::new(Full::new(Bytes::from(
            borrowed_val.finalize().to_string(),
        ))))
    } else {
        Ok(hyper::Response::new(Full::new(Bytes::from_static(
            b"Only queries are suppored",
        ))))
    }
}

use crate::ir::eval_ctx::EvalContext;
use crate::ir::eval_http::EvalHttp;
use crate::ir::IO;
use crate::request_context::CacheErr;
use crate::value::Value;
use std::num::NonZeroU64;

pub async fn eval_io(io: &IO, ctx: &mut EvalContext<'_>) -> anyhow::Result<Value> {
    let key = io.cache_key(ctx);

    if let Some(val) = ctx.request_ctx.cache.get(&key).await? {
        Ok(val.clone())
    } else {
        let val = eval_io_inner(io, ctx).await?;
        ctx.request_ctx
            .cache
            .set(key, val.clone(), NonZeroU64::MAX)
            .await?;
        Ok(val)
    }
}

async fn eval_io_inner(io: &IO, ctx: &mut EvalContext<'_>) -> Result<Value, CacheErr> {
    match io {
        IO::Http { req_template, .. } => {
            let eval_http = EvalHttp::new(ctx, req_template);
            let request = eval_http.init_request()?;
            let response = eval_http.execute(request).await?;

            Ok(response.body)
        }
    }
}

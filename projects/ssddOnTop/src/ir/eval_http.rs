use crate::http::response::Response;
use crate::http::RequestTemplate;
use crate::ir::eval_ctx::EvalContext;
use crate::value::Value;
use reqwest::Request;

pub struct EvalHttp<'a, 'ctx> {
    evaluation_ctx: &'ctx EvalContext<'a>,
    request_template: &'a RequestTemplate,
}

impl<'a, 'ctx> EvalHttp<'a, 'ctx> {
    pub fn new(
        evaluation_ctx: &'ctx EvalContext<'a>,
        request_template: &'a RequestTemplate,
    ) -> Self {
        Self {
            evaluation_ctx,
            request_template,
        }
    }
    pub fn init_request(&self) -> anyhow::Result<Request> {
        self.request_template.to_request(self.evaluation_ctx)
    }

    pub async fn execute(&self, req: Request) -> anyhow::Result<Response<Value>> {
        let ctx = &self.evaluation_ctx;
        let response = execute_raw_request(ctx, req).await?;

        Ok(response)
    }
}

pub async fn execute_raw_request(
    ctx: &EvalContext<'_>,
    req: Request,
) -> anyhow::Result<Response<Value>> {
    let response = ctx
        .request_ctx
        .runtime
        .http
        .execute(req)
        .await?
        .to_serde_json()?;

    let resp = Response {
        status: response.status,
        headers: response.headers,
        body: Value::new(response.body),
    };
    Ok(resp)
}

use crate::request_context::RequestContext;
use crate::value::Value;
use std::borrow::Cow;

#[derive(Clone)]
pub struct EvalContext<'a> {
    // Context create for each GraphQL Request
    pub request_ctx: &'a RequestContext,

    // graphql_ctx: &'a Ctx,
    pub graphql_ctx_value: Option<Value>,

    graphql_ctx_args: Option<Value>,
}

impl<'a> EvalContext<'a> {
    pub fn new(request_ctx: &'a RequestContext) -> Self {
        Self {
            request_ctx,
            graphql_ctx_value: None,
            graphql_ctx_args: None,
        }
    }
    pub fn with_value(self, value: Value) -> Self {
        Self {
            graphql_ctx_value: Some(value),
            ..self
        }
    }
    pub fn with_args(self, args: Value) -> Self {
        Self {
            graphql_ctx_args: Some(args),
            ..self
        }
    }
    pub fn path_arg<T: AsRef<str>>(&self, path: &[T]) -> Option<Cow<'a, Value>> {
        let args = self.graphql_ctx_args.as_ref()?;
        get_path_value(args, path).map(|a| Cow::Owned(a.clone()))
    }

    pub fn path_value<T: AsRef<str>>(&self, path: &[T]) -> Option<Cow<'a, Value>> {
        // TODO: add unit tests for this
        if let Some(value) = self.graphql_ctx_value.as_ref() {
            get_path_value(value, path).map(Cow::Owned)
        } else {
            Some(Cow::Owned(Value::new(serde_json::Value::Null)))
            // get_path_value(self.graphql_ctx.value()?, path).map(Cow::Borrowed)
        }
    }
}

pub fn get_path_value<T: AsRef<str>>(input: &Value, path: &[T]) -> Option<Value> {
    let mut value = Some(input.serde());
    for name in path {
        match value {
            Some(serde_json::Value::Object(map)) => {
                value = map.get(name.as_ref());
            }

            Some(serde_json::Value::Array(list)) => {
                value = list.get(name.as_ref().parse::<usize>().ok()?);
            }
            _ => return None,
        }
    }

    value.map(|v| Value::new(v.clone()))
}

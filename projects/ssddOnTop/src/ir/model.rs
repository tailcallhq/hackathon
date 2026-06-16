use crate::http;
use crate::ir::eval_ctx::EvalContext;
use crate::ir::eval_io::eval_io;
use crate::value::Value;
use std::num::NonZeroU64;
// use crate::jit::eval_ctx::EvalContext;

#[derive(Clone, Debug)]
pub enum IR {
    IO(IO),
    Cache(Cache),
}

#[derive(Clone, Debug)]
pub struct Cache {
    pub max_age: NonZeroU64,
    pub io: IO,
}

#[derive(Clone, Copy, Debug)]
pub struct DataLoaderId(usize);

impl DataLoaderId {
    pub fn new(id: usize) -> Self {
        Self(id)
    }

    pub fn as_usize(&self) -> usize {
        self.0
    }
}

#[derive(PartialEq, Eq, Clone, Hash, Debug)]
pub struct IoId(u64);

impl IoId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug)]
pub enum IO {
    Http { req_template: http::RequestTemplate },
}

impl IR {
    pub async fn eval<'a, 'b>(&'a self, ctx: &'b mut EvalContext<'a>) -> anyhow::Result<Value> {
        match self {
            IR::IO(io) => eval_io(io, ctx).await,
            IR::Cache(_) => todo!(),
        }
    }
}

impl<'a> IO {
    pub fn cache_key(&self, ctx: &EvalContext<'a>) -> IoId {
        match self {
            IO::Http { req_template, .. } => req_template.cache_key(ctx),
        }
    }
}

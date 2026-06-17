#![allow(unused, non_snake_case)]
mod app_ctx;
mod blueprint;
mod cache;
mod config;
mod directive;
mod dl;
mod endpoint;
mod from_doc;
mod hasher;
mod helpers;
mod http;
mod ir;
mod jit;
mod json;
mod mustache;
mod path;
mod request_context;
pub mod run;
mod target_runtime;
mod value;

pub fn is_default<T: Default + Eq>(val: &T) -> bool {
    *val == T::default()
}

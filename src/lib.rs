mod benchmarks;
mod command;
mod graphql_tests;
pub mod project;
mod request;
mod utils;
mod introspection;
mod query_info;
mod type_info;

pub const ROOT_DIR: &str = env!("CARGO_MANIFEST_DIR");

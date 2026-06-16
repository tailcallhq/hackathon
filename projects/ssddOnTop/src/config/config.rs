use crate::blueprint::wrapping_type;
use crate::config::url_query::URLQuery;
use crate::directive::DirectiveCodec;
use crate::http::method::Method;
use crate::is_default;
use async_graphql::parser::types::ConstDirective;
use async_graphql::Positioned;
use macros::{DirectiveDefinition, InputDefinition};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::num::NonZeroU64;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    pub types: BTreeMap<String, Type1>,
    pub upstream: Upstream,
    pub server: Server,
    pub schema: RootSchema,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RootSchema {
    pub query: Option<String>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub mutation: Option<String>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub subscription: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Server {
    #[serde(default, skip_serializing_if = "is_default")]
    pub port: u16,
}

impl Default for Server {
    fn default() -> Self {
        Server { port: 8000 }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Upstream {
    #[serde(rename = "baseURL", default, skip_serializing_if = "is_default")]
    pub base_url: Option<String>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub http_cache: Option<u64>,
}

// TODO: rename
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
pub struct Type1 {
    pub fields: BTreeMap<String, Field>,
    pub cache: Option<Cache>,
}

impl Type1 {
    pub fn fields(mut self, fields: Vec<(&str, Field)>) -> Self {
        let mut graphql_fields = BTreeMap::new();
        for (name, field) in fields {
            graphql_fields.insert(name.to_string(), field);
        }
        self.fields = graphql_fields;
        self
    }

    pub fn scalar(&self) -> bool {
        self.fields.is_empty()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Cache {
    pub max_age: NonZeroU64,
}

#[derive(
    Serialize,
    Deserialize,
    Clone,
    Debug,
    PartialEq,
    Eq,
    schemars::JsonSchema,
    macros::CustomResolver,
)]
pub enum Resolver {
    Http(Http),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
pub struct Field {
    pub ty_of: wrapping_type::Type,
    #[serde(flatten, default, skip_serializing_if = "is_default")]
    pub resolver: Option<Resolver>,
    pub args: BTreeMap<String, Arg>,
}

#[derive(
    Serialize,
    Deserialize,
    Clone,
    Debug,
    PartialEq,
    Eq,
    schemars::JsonSchema,
    DirectiveDefinition,
    InputDefinition,
)]
#[directive_definition(locations = "FieldDefinition")]
pub struct Http {
    pub path: String,
    #[serde(default, skip_serializing_if = "is_default")]
    pub method: Method,
    #[serde(rename = "baseURL", default, skip_serializing_if = "is_default")]
    pub base_url: Option<String>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub query: Vec<URLQuery>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Arg {
    pub type_of: wrapping_type::Type,
    pub default_value: Option<serde_json::Value>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum GraphQLOperationType {
    #[default]
    Query,
    Mutation,
}

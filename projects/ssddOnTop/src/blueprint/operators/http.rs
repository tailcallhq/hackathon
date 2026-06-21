use crate::blueprint::FieldDefinition;
use crate::config;
use crate::config::Resolver;
use crate::endpoint::Endpoint;
use crate::http::method::Method;
use crate::http::RequestTemplate;
use crate::ir::{IO, IR};

fn compile_http(config_module: &config::Config, http: &config::Http) -> anyhow::Result<IR> {
    let mut base_url = String::new();
    if let Some(base) = http.base_url.clone() {
        base_url = base;
    } else if let Some(base) = config_module.upstream.base_url.clone() {
        base_url = base;
    } else {
        return Err(anyhow::anyhow!("No base URL defined"));
    }
    let mut base_url = base_url.trim_end_matches('/').to_owned();
    base_url.push_str(http.path.clone().as_str());

    let query = http
        .query
        .clone()
        .iter()
        .map(|key_value| {
            (
                key_value.key.clone(),
                key_value.value.clone(),
                key_value.skip_empty.unwrap_or_default(),
            )
        })
        .collect();

    let req_template = RequestTemplate::try_from(
        Endpoint::new(base_url.to_string())
            .method(http.method.clone())
            .query(query),
    )?;

    let ir = if http.method == Method::GET {
        IR::IO(IO::Http { req_template })
    } else {
        IR::IO(IO::Http { req_template })
    };

    Ok(ir)
}

pub fn update_http(
    field: &config::Field,
    config: &config::Config,
    mut def: FieldDefinition,
) -> anyhow::Result<FieldDefinition> {
    let Some(Resolver::Http(http)) = field.resolver.as_ref() else {
        return Ok(def);
    };

    let ir = compile_http(config, http)?;
    def.resolver = Some(ir);
    // TODO: Validate
    Ok(def)
}

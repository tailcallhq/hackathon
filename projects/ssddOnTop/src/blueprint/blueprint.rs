use crate::blueprint::definitions::to_definitions;
use crate::blueprint::model::{Arg, ArgId, Field, FieldId, FieldName, TypeName};
use crate::blueprint::wrapping_type::Type;
use crate::config::{Config, RootSchema};
use crate::ir::IR;
use derive_setters::Setters;
use serde_json::Value;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};

#[derive(Debug, Eq, Hash, PartialEq)]
pub struct FieldHash {
    pub name: FieldName,
    pub id: TypeName,
}

impl FieldHash {
    pub fn new(name: FieldName, id: TypeName) -> Self {
        Self { name, id }
    }
}

#[derive(Debug)]
pub struct Blueprint {
    pub fields: HashMap<FieldHash, Field>,
    pub server: Server,
    pub upstream: Upstream,
    pub schema: RootSchema,
}

#[derive(Clone, Debug)]
pub struct Directive {
    pub name: String,
    pub arguments: HashMap<String, Value>,
    pub index: usize,
}

#[derive(Clone, Debug)]
pub enum Definition {
    Interface(InterfaceTypeDefinition),
    Object(ObjectTypeDefinition),
    InputObject(InputObjectTypeDefinition),
}

#[derive(Clone, Debug)]
pub struct InputObjectTypeDefinition {
    pub name: String,
    pub fields: Vec<InputFieldDefinition>,
}

#[derive(Clone, Debug)]
pub struct ObjectTypeDefinition {
    pub name: String,
    pub fields: Vec<FieldDefinition>,
}

#[derive(Clone, Debug)]
pub struct InterfaceTypeDefinition {
    pub name: String,
    pub fields: Vec<FieldDefinition>,
}

#[derive(Clone, Debug, Setters, Default)]
pub struct FieldDefinition {
    pub name: String,
    pub args: Vec<InputFieldDefinition>,
    pub of_type: Type,
    pub resolver: Option<IR>,
    pub directives: Vec<Directive>,
}

#[derive(Clone, Debug)]
pub struct InputFieldDefinition {
    pub name: String,
    pub of_type: Type,
}

#[derive(Clone, Debug)]
pub struct Server {
    pub host: IpAddr,
    pub port: u16,
}

impl Default for Server {
    fn default() -> Self {
        Self {
            host: IpAddr::from([127, 0, 0, 1]),
            port: 8000,
        }
    }
}

impl Server {
    pub fn addr(&self) -> SocketAddr {
        SocketAddr::new(self.host, self.port)
    }
}
#[derive(Clone, Debug, Default)]
pub struct Upstream {
    pub base_url: Option<String>,
    pub http_cache: u64,
}

impl TryFrom<&Config> for Blueprint {
    type Error = anyhow::Error;

    fn try_from(config: &Config) -> Result<Self, Self::Error> {
        let qry = config
            .schema
            .query
            .as_ref()
            .ok_or(anyhow::anyhow!("Query not found"))?;
        let defs = to_definitions(config)?;

        let fields = fields_to_map(qry, config, defs);

        let server = Server {
            host: IpAddr::from([127, 0, 0, 1]),
            port: config.server.port,
        };
        let upstream = Upstream {
            base_url: config.upstream.base_url.clone(),
            http_cache: config.upstream.http_cache.unwrap_or(10000),
        };
        Ok(Blueprint {
            fields,
            server,
            upstream,
            schema: config.schema.clone(),
        })
    }
}

fn fields_to_map(qry: &str, config: &Config, defs: Vec<Definition>) -> HashMap<FieldHash, Field> {
    let mut fields = HashMap::new();
    populate_nested_field(config, qry, &mut fields, &defs);
    fields
}

fn populate_nested_field(
    config: &Config,
    ty_name: &str,
    field_map: &mut HashMap<FieldHash, Field>,
    defs: &[Definition],
) {
    // I don't have additional check for scalars as of now..
    // This should work fine
    if let Some(ty) = config.types.get(ty_name) {
        for (field_name, field) in ty.fields.iter() {
            let field_name = FieldName(field_name.clone());
            populate_nested_field(config, field.ty_of.name(), field_map, defs);
            let field = Field {
                name: field_name.clone(),
                type_of: field.ty_of.clone(),
                ir: {
                    let x = defs.iter().find_map(|def| match def {
                        Definition::Interface(int) => Some(
                            int.fields
                                .iter()
                                .find(|f| field_name.0.eq(&f.name) && int.name.eq(ty_name))?
                                .clone(),
                        ),
                        Definition::Object(obj) => Some(
                            obj.fields
                                .iter()
                                .find(|f| field_name.0.eq(&f.name) && obj.name.eq(ty_name))?
                                .clone(),
                        ),
                        Definition::InputObject(_) => None,
                    });
                    // println!("resolver for: {} is {:?}", field_name.0, x);
                    
                    x.and_then(|x| x.resolver.clone())
                },
                args: field
                    .args
                    .iter()
                    .map(|(arg_name, arg)| {
                        let arg = Arg {
                            name: arg_name.clone(),
                            type_of: arg.type_of.clone(),
                        };
                        arg
                    })
                    .collect(),
            };

            field_map.insert(
                FieldHash {
                    name: field_name,
                    id: TypeName(ty_name.to_string()),
                },
                field,
            );
        }
    }
}

#[cfg(test)]
mod test {
    use crate::config::ConfigReader;

    #[test]
    fn test() {
        let reader = ConfigReader::init();
        let root = env!("CARGO_MANIFEST_DIR");
        let config = reader
            .read(format!("{}/schema/schema.graphql", root))
            .unwrap();
        let blueprint = crate::blueprint::Blueprint::try_from(&config).unwrap();
        println!("{:#?}", blueprint.fields );
    }
}

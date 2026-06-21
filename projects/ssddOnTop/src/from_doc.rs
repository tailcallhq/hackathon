use crate::config::{Arg, Cache, Config, Field, Resolver, RootSchema, Server, Type1, Upstream};
use crate::directive::DirectiveCodec;
use anyhow::Result;
use async_graphql::parser::types::{
    ConstDirective, FieldDefinition, InputObjectType, InputValueDefinition, InterfaceType,
    ObjectType, SchemaDefinition, ServiceDocument, Type, TypeDefinition, TypeKind,
    TypeSystemDefinition,
};
use async_graphql::{Name, Positioned};
use std::collections::BTreeMap;

const DEFAULT_SCHEMA_DEFINITION: &SchemaDefinition = &SchemaDefinition {
    extend: false,
    directives: Vec::new(),
    query: None,
    mutation: None,
    subscription: None,
};

pub fn from_doc(doc: ServiceDocument) -> Result<Config> {
    let type_definitions: Vec<_> = doc
        .definitions
        .iter()
        .filter_map(|def| match def {
            TypeSystemDefinition::Type(td) => Some(td),
            _ => None,
        })
        .collect();

    let types = to_types(&type_definitions)?;
    let sd = schema_definition(&doc)?;
    let server = server(sd)?;
    let upstream = upstream(sd)?;
    let schema = schema_definition(&doc).map(to_root_schema)?;

    Ok(Config {
        types,
        upstream,
        server,
        schema,
    })
}
fn to_root_schema(schema_definition: &SchemaDefinition) -> RootSchema {
    let query = schema_definition.query.as_ref().map(pos_name_to_string);
    let mutation = schema_definition.mutation.as_ref().map(pos_name_to_string);
    let subscription = schema_definition
        .subscription
        .as_ref()
        .map(pos_name_to_string);

    RootSchema {
        query,
        mutation,
        subscription,
    }
}
fn upstream(schema_definition: &SchemaDefinition) -> Result<Upstream> {
    process_schema_directives(schema_definition, Upstream::directive_name().as_str())
}

fn schema_definition(doc: &ServiceDocument) -> Result<&SchemaDefinition> {
    doc.definitions
        .iter()
        .find_map(|def| match def {
            TypeSystemDefinition::Schema(schema_definition) => Some(&schema_definition.node),
            _ => None,
        })
        .map_or_else(|| Ok(DEFAULT_SCHEMA_DEFINITION), Ok)
}

fn server(schema_definition: &SchemaDefinition) -> Result<Server> {
    process_schema_directives(schema_definition, Server::directive_name().as_str())
}

fn process_schema_directives<T: DirectiveCodec + Default>(
    schema_definition: &SchemaDefinition,
    directive_name: &str,
) -> Result<T> {
    let mut res = Ok(T::default());
    for directive in schema_definition.directives.iter() {
        if directive.node.name.node.as_ref() == directive_name {
            res = T::from_directive(&directive.node);
        }
    }
    res
}

fn pos_name_to_string(pos: &Positioned<Name>) -> String {
    pos.node.to_string()
}

fn to_types(
    type_definitions: &Vec<&Positioned<TypeDefinition>>,
) -> Result<BTreeMap<String, Type1>> {
    let mut map = BTreeMap::new();
    for type_definition in type_definitions.iter() {
        let type_name = pos_name_to_string(&type_definition.node.name);
        let ty = match type_definition.node.kind.clone() {
            TypeKind::Object(object_type) => {
                to_object_type(&object_type, &type_definition.node.directives)
            }
            TypeKind::Interface(interface_type) => {
                to_object_type(&interface_type, &type_definition.node.directives)
            }
            TypeKind::InputObject(input_object_type) => to_input_object(input_object_type),
            _ => Err(anyhow::anyhow!(
                "Unsupported type kind: {:?}",
                type_definition.node.kind
            )),
        }?;

        map.insert(type_name, ty);
    }

    Ok(map)
}

fn to_input_object(input_object_type: InputObjectType) -> Result<Type1> {
    let fields = to_input_object_fields(&input_object_type.fields)?;
    Ok(Type1 {
        fields,
        ..Default::default()
    })
}

fn to_input_object_fields(
    input_object_fields: &Vec<Positioned<InputValueDefinition>>,
) -> Result<BTreeMap<String, Field>> {
    to_fields_inner(input_object_fields, to_input_object_field)
}

fn to_input_object_field(field_definition: &InputValueDefinition) -> Result<Field> {
    to_common_field(field_definition, BTreeMap::new())
}

fn to_object_type<T>(object: &T, directives: &[Positioned<ConstDirective>]) -> Result<Type1>
where
    T: ObjectLike,
{
    let fields = object.fields();

    let cache = Cache::from_directives(directives.iter())?;
    let fields = to_fields(fields)?;
    Ok(Type1 { fields, cache })
}

fn to_fields(fields: &Vec<Positioned<FieldDefinition>>) -> Result<BTreeMap<String, Field>> {
    to_fields_inner(fields, to_field)
}

fn to_fields_inner<T, F>(
    fields: &Vec<Positioned<T>>,
    transform: F,
) -> Result<BTreeMap<String, Field>>
where
    F: Fn(&T) -> Result<Field>,
    T: HasName,
{
    let mut map = BTreeMap::new();
    for field in fields.iter() {
        let field_name = pos_name_to_string(field.node.name());
        let (name, field) = transform(&field.node).map(|field| (field_name, field))?;
        map.insert(name, field);
    }

    Ok(map)
}

fn to_field(field_definition: &FieldDefinition) -> Result<Field> {
    to_common_field(field_definition, to_args(field_definition))
}

fn to_common_field<F>(field: &F, args: BTreeMap<String, Arg>) -> Result<Field>
where
    F: FieldLike + HasName,
{
    let type_of = field.type_of();
    let directives = field.directives();

    let resolver = Resolver::from_directives(directives)?;
    Ok(Field {
        ty_of: type_of.into(),
        args,
        resolver,
    })
}

fn to_args(field_definition: &FieldDefinition) -> BTreeMap<String, Arg> {
    let mut args = BTreeMap::new();

    for arg in field_definition.arguments.iter() {
        let arg_name = pos_name_to_string(&arg.node.name);
        let arg_val = to_arg(&arg.node);
        args.insert(arg_name, arg_val);
    }

    args
}

fn to_arg(input_value_definition: &InputValueDefinition) -> Arg {
    let type_of = &input_value_definition.ty.node;

    let default_value = if let Some(pos) = input_value_definition.default_value.as_ref() {
        let value = &pos.node;
        serde_json::to_value(value).ok()
    } else {
        None
    };
    Arg {
        type_of: type_of.into(),
        default_value,
    }
}

trait HasName {
    fn name(&self) -> &Positioned<Name>;
}
impl HasName for FieldDefinition {
    fn name(&self) -> &Positioned<Name> {
        &self.name
    }
}
impl HasName for InputValueDefinition {
    fn name(&self) -> &Positioned<Name> {
        &self.name
    }
}

trait FieldLike {
    fn type_of(&self) -> &Type;
    fn description(&self) -> &Option<Positioned<String>>;
    fn directives(&self) -> &[Positioned<ConstDirective>];
}
impl FieldLike for FieldDefinition {
    fn type_of(&self) -> &Type {
        &self.ty.node
    }
    fn description(&self) -> &Option<Positioned<String>> {
        &self.description
    }
    fn directives(&self) -> &[Positioned<ConstDirective>] {
        &self.directives
    }
}
impl FieldLike for InputValueDefinition {
    fn type_of(&self) -> &Type {
        &self.ty.node
    }
    fn description(&self) -> &Option<Positioned<String>> {
        &self.description
    }
    fn directives(&self) -> &[Positioned<ConstDirective>] {
        &self.directives
    }
}
trait ObjectLike {
    fn fields(&self) -> &Vec<Positioned<FieldDefinition>>;
}
impl ObjectLike for ObjectType {
    fn fields(&self) -> &Vec<Positioned<FieldDefinition>> {
        &self.fields
    }
}
impl ObjectLike for InterfaceType {
    fn fields(&self) -> &Vec<Positioned<FieldDefinition>> {
        &self.fields
    }
}

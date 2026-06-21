use crate::blueprint::operators::http::update_http;
use crate::blueprint::{
    Definition, FieldDefinition, InputFieldDefinition, InputObjectTypeDefinition,
    InterfaceTypeDefinition, ObjectTypeDefinition,
};
use crate::config;
use crate::config::Config;

pub fn to_definitions(config: &Config) -> anyhow::Result<Vec<Definition>> {
    let mut definitions = vec![];
    for (ty_name, ty) in config.types.iter() {
        let def =
            to_object_type_definition(ty_name, ty, config).map(
                |definition| match definition {
                    Definition::Object(_) => {
                        definition
                        /*if config.input_types().contains(ty_name) {
                            to_input_object_type_definition(object_type_definition)
                        } else if config.interface_types().contains(ty_name) {
                            to_interface_type_definition(object_type_definition)
                        } else {
                            Ok(definition)
                        }*/
                    }
                    _ => definition,
                },
            )?;
        definitions.push(def);
    }
    Ok(definitions)
}

fn to_interface_type_definition(definition: ObjectTypeDefinition) -> anyhow::Result<Definition> {
    Ok(Definition::Interface(InterfaceTypeDefinition {
        name: definition.name,
        fields: definition.fields,
    }))
}

fn to_input_object_type_definition(definition: ObjectTypeDefinition) -> anyhow::Result<Definition> {
    Ok(Definition::InputObject(InputObjectTypeDefinition {
        name: definition.name,
        fields: definition
            .fields
            .iter()
            .map(|field| InputFieldDefinition {
                name: field.name.clone(),
                of_type: field.of_type.clone(),
            })
            .collect(),
    }))
}

fn to_object_type_definition(
    name: &str,
    type_of: &config::Type1,
    config_module: &Config,
) -> anyhow::Result<Definition> {
    to_fields(name, type_of, config_module).map(|fields| {
        Definition::Object(ObjectTypeDefinition {
            name: name.to_string(),
            fields,
        })
    })
}

fn to_fields(
    name: &str,
    ty: &config::Type1,
    config: &Config,
) -> anyhow::Result<Vec<FieldDefinition>> {
    if !config.types.contains_key(name) {
        // assume it's a scalar
        return Ok(vec![]);
    }

    let mut fields = vec![];
    for (field_name, field) in ty.fields.iter() {
        to_field_definition(field_name, field, config).map(|field| {
            fields.push(field);
        })?;
    }

    Ok(fields)
}

fn to_field_definition(
    field_name: &str,
    field: &config::Field,
    config: &Config,
) -> anyhow::Result<FieldDefinition> {
    let mut def = FieldDefinition::default();
    def = update_args(field_name, field.clone(), def);
    def = update_http(field, config, def)?;
    Ok(def)
}

fn update_args(
    field_name: &str,
    field: config::Field,
    mut def: FieldDefinition,
) -> FieldDefinition {
    let args = field
        .args
        .iter()
        .map(|(name, arg)| {
            
            InputFieldDefinition {
                name: name.clone(),
                of_type: arg.type_of.clone(),
            }
        })
        .collect::<Vec<_>>();

    def.name = field_name.to_string();
    def.args = args;
    def.of_type = field.ty_of.clone();

    def
}

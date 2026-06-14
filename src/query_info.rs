use serde::{Deserialize, Serialize, Serializer, Deserializer};
use serde::ser::SerializeStruct;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Schema {
    pub data: SchemaData,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct SchemaData {
    #[serde(rename = "__schema")]
    pub schema: SchemaType,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SchemaType {
    pub query_type: QueryType,
}

#[derive(Debug, PartialEq, Clone)]
pub struct QueryType {
    pub fields: Vec<Field>,
}

impl Serialize for QueryType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut fields = self.fields.clone();
        fields.sort_by(|a, b| a.name.cmp(&b.name));
        let mut state = serializer.serialize_struct("queryType", 1)?;
        state.serialize_field("fields", &fields)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for QueryType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct QueryTypeHelper {
            fields: Vec<Field>,
        }

        let helper = QueryTypeHelper::deserialize(deserializer)?;
        let mut fields = helper.fields;
        fields.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(QueryType { fields })
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Field {
    pub name: String,
    pub field_type: FieldType,
    pub args: Vec<Argument>,
}

impl Serialize for Field {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut args = self.args.clone();
        args.sort_by(|a, b| a.name.cmp(&b.name));
        let mut state = serializer.serialize_struct("field", 3)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("type", &self.field_type)?;
        state.serialize_field("args", &args)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for Field {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct FieldHelper {
            name: String,
            #[serde(rename = "type")]
            field_type: FieldType,
            args: Vec<Argument>,
        }

        let helper = FieldHelper::deserialize(deserializer)?;
        let mut args = helper.args;
        args.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(Field {
            name: helper.name,
            field_type: helper.field_type,
            args,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Argument {
    pub name: String,
    #[serde(rename = "type")]
    pub arg_type: FieldType,
    pub default_value: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FieldType {
    pub kind: String,
    pub name: Option<String>,
    pub of_type: Option<Box<FieldType>>,
}


impl FieldType {
    pub fn get_name(&self) -> Option<String> {
        match &self.name {
            Some(name) if !name.is_empty() => Some(name.clone()),
            _ => self.of_type.as_ref().and_then(|t| t.get_name()),
        }
    }
}
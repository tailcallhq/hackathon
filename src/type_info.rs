use serde::{ser::SerializeStruct, Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    name: String,
    args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Type {
    name: String,
    kind: String,
    fields: Vec<Field>,
}


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Data {
    #[serde(rename = "__type")]
    type_info: Type,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Root {
    data: Data,
}

// Custom serialization for Field
impl Serialize for Field {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Field", 2)?;
        state.serialize_field("name", &self.name)?;
        let mut sorted_args = self.args.clone();
        sorted_args.sort();
        state.serialize_field("args", &sorted_args)?;
        state.end()
    }
}

// Custom deserialization for Field
impl<'de> Deserialize<'de> for Field {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct FieldHelper {
            name: String,
            args: Vec<String>,
        }

        let helper = FieldHelper::deserialize(deserializer)?;
        let mut args = helper.args;
        args.sort();

        Ok(Field {
            name: helper.name,
            args,
        })
    }
}

// Custom serialization for Type
impl Serialize for Type {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Type", 3)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("kind", &self.kind)?;
        let mut sorted_fields = self.fields.clone();
        sorted_fields.sort_by(|a, b| a.name.cmp(&b.name));
        state.serialize_field("fields", &sorted_fields)?;
        state.end()
    }
}

// Custom deserialization for Type
impl<'de> Deserialize<'de> for Type {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct TypeHelper {
            name: String,
            kind: String,
            fields: Vec<Field>,
        }

        let helper = TypeHelper::deserialize(deserializer)?;
        let mut fields = helper.fields;
        fields.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(Type {
            name: helper.name,
            kind: helper.kind,
            fields,
        })
    }
}

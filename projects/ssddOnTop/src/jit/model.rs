use crate::blueprint::model::{FieldName, TypeName};
use crate::blueprint::{Blueprint, FieldHash};
use crate::ir::eval_ctx::EvalContext;
use crate::ir::IR;
use crate::json::JsonObjectLike;
use crate::value::Value;
use async_graphql::parser::types::{DocumentOperations, ExecutableDocument, OperationType, Selection, SelectionSet};
use async_graphql::Positioned;
use serde_json::Map;
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use serde_json_borrow::ObjectAsVec;

pub struct PathFinder<'a> {
    doc: ExecutableDocument,
    blueprint: &'a Blueprint,
}
#[derive(Debug)]
pub struct Fields {
    fields: Vec<Field>,
}

#[derive(Debug)]
pub struct Fields1<'a> {
    fields: Vec<Field1<'a>>,
}

fn to_borrowed(val: &Value) -> serde_json_borrow::Value {
    serde_json_borrow::Value::from(val.serde())
}

impl<'a> Fields1<'a> {
    #[inline(always)]
    pub fn finalize(&'a self) -> serde_json_borrow::Value<'a> {
        let mut map = ObjectAsVec::new();
        for field in self.fields.iter() {
            let name = field.name;
            let val = Self::finalize_inner(field, None, None);
            map.insert(name, val);
        }
        let mut ans = ObjectAsVec::new();
        ans.insert("data", serde_json_borrow::Value::Object(map));
        // map.insert("data".to_string(), self.finalize_inner());
        // serde_json::Value::Object(map)
        serde_json_borrow::Value::Object(ans)
    }
    #[inline(always)]
    fn finalize_inner(field: &'a Field1<'a>, mut value: Option<&'a serde_json_borrow::Value<'a>>, index: Option<usize>) -> serde_json_borrow::Value<'a> {
        if let Some(val) = &field.resolved {
            if value.is_none() {
                value = Some(val);
            }
        }
        if let Some(val) = value{
            match (val.as_array(), val.as_object()) {
                (_, Some(obj)) => {
                    // let mut ans = Map::new();
                    let mut ans = ObjectAsVec::new();

                    if field.nested.is_empty() {
                        let val = obj.get_key(field.name);
                        let value = Self::finalize_inner(field, val, index);
                        ans.insert(field.name, value);
                    } else {
                        for child in field.nested.iter() {
                            let child_name = child.name;
                            let val = obj.get_key(child.name);
                            let val = Self::finalize_inner(child, val, index);
                            ans.insert(child_name, val);
                        }
                    }

                    serde_json_borrow::Value::Object(ans)
                }
                (Some(arr), _) => {
                    if let Some(index) = index {
                        let val = arr.get(index);
                        let val = Self::finalize_inner(field, val, None);
                        val
                    } else {
                        let mut ans = vec![];
                        for (i, val) in arr.iter().enumerate() {
                            let val = Self::finalize_inner(field, Some(val), Some(i));
                            ans.push(val);
                        }
                        serde_json_borrow::Value::Array(ans)
                    }
                }
                _ => value.cloned().unwrap_or_default(),
            }
        } else {
            serde_json_borrow::Value::Null
        }
    }
}

#[derive(Debug)]
// TODO: give it a lifetime
// it won't make much difference..
// but anyways
pub struct Field {
    ir: Option<IR>,
    pub name: String,
    pub type_of: crate::blueprint::wrapping_type::Type,
    nested: Vec<Field>,
    pub args: Option<Value>,
    pub resolved: Option<Value>,
}

impl Fields {
    #[inline(always)]
    pub fn to_borrowed<'a>(&'a self) -> Fields1<'a> {
        let fields = Self::borrowed_inner(&self.fields);
        Fields1 {
            fields,
        }
    }

    #[inline(always)]
    pub fn borrowed_inner<'a>(vec: &'a [Field]) -> Vec<Field1<'a>> {
        let mut ans = vec![];
        for field in vec.iter() {
            let field = Field1 {
                ir: field.ir.as_ref(),
                name: field.name.as_str(),
                type_of: &field.type_of,
                nested: Self::borrowed_inner(&field.nested),
                args: field.args.as_ref(),
                resolved: field.resolved.as_ref().map(|v| serde_json_borrow::Value::from(v.serde())),
            };
            ans.push(field);
        }
        ans
    }

}

#[derive(Debug)]
pub struct Field1<'a> {
    ir: Option<&'a IR>,
    pub name: &'a str,
    pub type_of: &'a crate::blueprint::wrapping_type::Type,
    nested: Vec<Field1<'a>>,
    pub args: Option<&'a Value>,
    pub resolved: Option<serde_json_borrow::Value<'a>>,
}
impl Fields {
    #[inline(always)]
    pub async fn resolve<'a>(mut self, eval_context: EvalContext<'a>) -> anyhow::Result<Fields> {
        let mut ans = vec![];
        ans = Self::resolve_inner(self.fields, eval_context, None).await?;
        Ok(Fields {
            fields: ans,
        })
    }

    #[inline(always)]
    fn resolve_inner<'a>(fields: Vec<Field>, mut eval_context: EvalContext<'a>, parent: Option<Value>) -> Pin<Box<dyn Future<Output=anyhow::Result<Vec<Field>>> + Send + 'a>> {
        Box::pin(async move {
            let mut ans = vec![];
            for mut field in fields {
                let mut parent_val = None;

                if let Some(ir) = field.ir.as_ref() {
                    if let Some(val) = field.args.clone() {
                        eval_context = eval_context.with_args(val);
                    }

                    let val = match &parent {
                        Some(val) => {
                            match val.serde() {
                                serde_json::Value::Array(arr) => {
                                    let mut ans = vec![];
                                    for val in arr {
                                        eval_context = eval_context.with_value(Value::new(val.clone()));
                                        let val = ir.eval(&mut eval_context.clone()).await?;
                                        ans.push(val.into_serde());
                                    }
                                    Some(Value::new(serde_json::Value::Array(ans)))
                                }
                                val => {
                                    eval_context = eval_context.with_value(Value::new(val.clone()));
                                    let val = ir.eval(&mut eval_context.clone()).await?;
                                    Some(val)
                                }
                            }
                        }
                        None => {
                            let val = ir.eval(&mut eval_context.clone()).await?;
                            Some(val)
                        }
                    };
                    parent_val = val.clone();
                    field.resolved = val;
                } else {
                    // println!("hx: {}", field.name);
                    // let val = Self::resolve_non_ir(eval_context.graphql_ctx_value.as_ref().map(|v| v.serde()).unwrap_or(&serde_json::Value::Null), field.name.as_str());
                    // println!("{}",val);
                    // let val = eval_context.path_value(&[field.name.as_str()]);
                    // let val = val.unwrap_or(Cow::Owned(Value::new(serde_json::Value::Null))).into_owned();
                    // field.resolved = Some(Value::new(val));
                }

                let eval_ctx_clone = eval_context.clone();
                field.nested = Self::resolve_inner(field.nested, eval_ctx_clone, parent_val).await?;
                ans.push(field);
            }
            Ok(ans)
        })
    }
    #[inline(always)]
    fn resolve_non_ir(value: &serde_json::Value, key: &str) -> serde_json::Value {
        match value {
            serde_json::Value::Array(arr) => {
                let mut ans = vec![];
                for val in arr {
                    ans.push(Self::resolve_non_ir(val, key));
                }
                serde_json::Value::Array(ans)
            }
            serde_json::Value::Object(obj) => {
                let mut ans = Map::new();
                obj.get_key(key).map(|v| ans.insert(key.to_string(), v.clone())).unwrap_or_default();
                serde_json::Value::Object(ans)
            }
            val => val.clone(),
        }
    }
}

pub struct Holder<'a> {
    field_name: &'a str,
    field_type: &'a str,
    args: Vec<(&'a str, Value)>,
}

impl<'a> PathFinder<'a> {
    pub fn new(doc: ExecutableDocument, blueprint: &'a Blueprint) -> Self {
        Self { doc, blueprint }
    }
    pub async fn exec(&'a self) -> Fields {
        match &self.doc.operations {
            DocumentOperations::Single(single) => {
                let operation = &single.node;
                let selection_set = &operation.selection_set.node;
                let ty = match &operation.ty {
                    OperationType::Query => {
                        let query = self.blueprint.schema.query.as_ref().map(|v| v.as_str());
                        query
                    }
                    OperationType::Mutation => None,
                    OperationType::Subscription => None,
                };
                if let Some(ty) = ty {
                    Fields {
                        fields: self.iter(selection_set, ty),
                    }
                } else {
                    Fields {
                        fields: vec![],
                    }
                }
            }
            DocumentOperations::Multiple(multi) => {
                let (_,single) = multi.iter().next().unwrap();
                let operation = &single.node;
                let selection_set = &operation.selection_set.node;
                let ty = match &operation.ty {
                    OperationType::Query => {
                        let query = self.blueprint.schema.query.as_ref().map(|v| v.as_str());
                        query
                    }
                    OperationType::Mutation => None,
                    OperationType::Subscription => None,
                };
                if let Some(ty) = ty {
                    Fields {
                        fields: self.iter(selection_set, ty),
                    }
                } else {
                    Fields {
                        fields: vec![],
                    }
                }
            }
        }
    }
    #[inline(always)]
    fn iter(
        &self,
        selection: &SelectionSet,
        type_condition: &str,
    ) -> Vec<Field> {
        let mut fields = vec![];
        for selection in &selection.items {
            match &selection.node {
                Selection::Field(Positioned { node: gql_field, .. }) => {
                    let field_name = gql_field.name.node.as_str();
                    let request_args = gql_field
                        .arguments
                        .iter()
                        .map(|(k, v)| (k.node.as_str().to_string(), v.node.to_owned().into_const().map(|v| v.into_json().ok()).flatten().unwrap()))
                        .collect::<Map<_, _>>();

                    if let Some(field_def) = self.blueprint.fields.get(&FieldHash::new(FieldName(field_name.to_string()), TypeName(type_condition.to_string()))) {
                        let type_of = field_def.type_of.clone();
                        let child_fields = self.iter(
                            &gql_field.selection_set.node,
                            type_of.name(),
                        );
                        let field = Field {
                            ir: field_def.ir.clone(),
                            name: field_def.name.as_ref().to_string(),
                            type_of,
                            nested: child_fields,
                            args: match request_args.is_empty() {
                                false => Some(Value::new(serde_json::Value::Object(request_args))),
                                true => None,
                            },
                            resolved: None,
                        };

                        fields.push(field);
                    }
                }
                _ => (),
            }
        }

        fields
    }
}
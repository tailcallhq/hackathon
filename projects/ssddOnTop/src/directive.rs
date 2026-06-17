use crate::blueprint;
use async_graphql::parser::types::ConstDirective;
use async_graphql::{Name, Pos, Positioned};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use serde_path_to_error::deserialize;

fn pos<A>(a: A) -> Positioned<A> {
    Positioned::new(a, Pos::default())
}

#[allow(dead_code)]
fn to_const_directive(directive: &blueprint::Directive) -> anyhow::Result<ConstDirective> {
    let mut arguments = Vec::new();
    for (k, v) in directive.arguments.iter() {
        let name = pos(Name::new(k.clone()));
        let x = serde_json::from_value(v.clone())
            .map(pos)
            .map(|value| (name, value))?;
        arguments.push(x);
    }
    Ok(ConstDirective {
        name: pos(Name::new(directive.name.clone())),
        arguments,
    })
}

pub trait DirectiveCodec: Sized {
    fn directive_name() -> String;
    fn from_directive(directive: &ConstDirective) -> anyhow::Result<Self>;
    fn to_directive(&self) -> ConstDirective;
    fn trace_name() -> String {
        format!("@{}", Self::directive_name())
    }
    fn from_directives<'a>(
        directives: impl Iterator<Item = &'a Positioned<ConstDirective>>,
    ) -> anyhow::Result<Option<Self>> {
        for directive in directives {
            if directive.node.name.node == Self::directive_name() {
                return Self::from_directive(&directive.node).map(Some);
            }
        }
        Ok(None)
    }
}
fn lower_case_first_letter(s: &str) -> String {
    if s.len() <= 2 {
        s.to_lowercase()
    } else if let Some(first_char) = s.chars().next() {
        first_char.to_string().to_lowercase() + &s[first_char.len_utf8()..]
    } else {
        s.to_string()
    }
}

impl<'a, A: Deserialize<'a> + Serialize + 'a> DirectiveCodec for A {
    fn directive_name() -> String {
        lower_case_first_letter(
            std::any::type_name::<A>()
                .split("::")
                .last()
                .unwrap_or_default(),
        )
    }

    fn from_directive(directive: &ConstDirective) -> anyhow::Result<A> {
        let mut map = Map::new();
        for (k, v) in directive.arguments.iter() {
            let (k, v) = serde_json::to_value(&v.node).map(|v| (k.node.as_str().to_string(), v))?;
            map.insert(k, v);
        }
        Ok(deserialize(Value::Object(map))?)
        /*        Valid::from_iter(directive.arguments.iter(), |(k, v)| {
            Valid::from(serde_json::to_value(&v.node).map_err(|e| {
                ValidationError::new(e.to_string())
                    .trace(format!("@{}", directive.name.node).as_str())
            }))
                .map(|v| (k.node.as_str().to_string(), v))
        })
            .map(|items| {
                items.iter().fold(Map::new(), |mut map, (k, v)| {
                    map.insert(k.clone(), v.clone());
                    map
                })
            })
            .and_then(|map| match deserialize(Value::Object(map)) {
                Ok(a) => Valid::succeed(a),
                Err(e) => Valid::from_validation_err(
                    ValidationError::from(e).trace(format!("@{}", directive.name.node).as_str()),
                ),
            })*/
    }

    fn to_directive(&self) -> ConstDirective {
        let name = Self::directive_name();
        let value = serde_json::to_value(self).unwrap();
        let default_map = &Map::new();
        let map = value.as_object().unwrap_or(default_map);

        let mut arguments = Vec::new();
        for (k, v) in map {
            arguments.push((
                pos(Name::new(k.clone())),
                pos(serde_json::from_value(v.to_owned()).unwrap()),
            ));
        }

        ConstDirective {
            name: pos(Name::new(name)),
            arguments,
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::blueprint::Directive;
    use crate::directive::{pos, to_const_directive};
    use async_graphql::parser::types::ConstDirective;
    use async_graphql_value::Name;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_to_const_directive() {
        let directive = Directive {
            name: "test".to_string(),
            arguments: vec![("a".to_string(), serde_json::json!(1.0))]
                .into_iter()
                .collect(),
            index: 0,
        };

        let const_directive: ConstDirective = to_const_directive(&directive).unwrap();
        let expected_directive: ConstDirective = ConstDirective {
            name: pos(Name::new("test")),
            arguments: vec![(pos(Name::new("a")), pos(async_graphql::Value::from(1.0)))]
                .into_iter()
                .collect(),
        };

        assert_eq!(
            format!("{:?}", const_directive),
            format!("{:?}", expected_directive)
        );
    }
}

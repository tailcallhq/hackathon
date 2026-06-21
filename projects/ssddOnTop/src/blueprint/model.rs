use crate::ir::IR;
use std::fmt::{Debug, Formatter};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct FieldName(pub String);

impl AsRef<str> for FieldName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct TypeName(pub String);

#[derive(Clone)]
pub struct ArgId(usize);

impl Debug for ArgId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ArgId {
    pub fn new(id: usize) -> Self {
        ArgId(id)
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct FieldId(usize);

impl Debug for FieldId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FieldId {
    pub fn new(id: usize) -> Self {
        FieldId(id)
    }
    pub fn as_usize(&self) -> usize {
        self.0
    }
}

#[derive(Clone, Debug)]
pub struct Arg {
    pub name: String,
    pub type_of: crate::blueprint::wrapping_type::Type,
}

#[derive(Clone, Debug)]
pub struct Nested(Vec<Field>);
#[derive(Clone, Debug)]
pub struct Flat(FieldId);

#[derive(Clone, Debug)]
pub struct Field {
    pub name: FieldName,
    pub type_of: crate::blueprint::wrapping_type::Type,
    pub ir: Option<IR>,
    pub args: Vec<Arg>,
}

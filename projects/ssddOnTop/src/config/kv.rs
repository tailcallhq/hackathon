use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default, Eq, PartialEq)]
pub struct KeyValue {
    pub key: String,
    pub value: String,
}

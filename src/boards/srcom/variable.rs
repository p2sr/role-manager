use std::collections::HashMap;
use serde::Deserialize;
use crate::boards::srcom::category::CategoryId;
use crate::boards::srcom::Link;

#[derive(Deserialize, Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct VariableId(pub String);

#[derive(Deserialize, Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct VariableValueId(pub String);

#[derive(Deserialize, Debug, Clone)]
pub struct Variable {
    pub id: VariableId,
    pub name: String,
    pub category: Option<CategoryId>,
    pub scope: VariableScope,
    pub mandatory: bool,
    #[serde(rename = "user-defined")]
    pub user_defined: bool,
    pub obsoletes: bool,
    pub values: VariableValues,
    #[serde(rename = "is-subcategory")]
    pub is_subcategory: bool,
    pub links: Option<Vec<Link>>
}

#[derive(Deserialize, Debug, Clone)]
pub struct VariableValue {
    pub label: String,
    pub rules: Option<String>,
    pub flags: Option<VariableValueFlags>
}

#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(tag = "type")]
pub enum VariableScope {
    #[serde(rename = "global")]
    Global,
    #[serde(rename = "full-game")]
    FullGame,
    #[serde(rename = "all-levels")]
    AllLevels,
    #[serde(rename = "single-level")]
    SingleLevel
}

#[derive(Deserialize, Debug, Clone)]
pub struct VariableValues {
    pub values: HashMap<VariableValueId, VariableValue>,
    pub default: Option<VariableValueId>
}

#[derive(Deserialize, Debug, Copy, Clone)]
pub struct VariableValueFlags {
    pub miscellaneous: Option<bool>
}

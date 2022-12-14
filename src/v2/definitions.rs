use crate::v2::{schema::Schema, DEFINITIONS_REF};

use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct Definitions(pub HashMap<String, Schema>);

impl Definitions {
    pub fn get(&self, key: impl AsRef<str>) -> Option<&Schema> {
        let key = key.as_ref().trim_start_matches(DEFINITIONS_REF);
        self.0.get(key)
    }
}

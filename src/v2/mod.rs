pub mod codegen;
pub mod definitions;
pub mod items;
pub mod operation;
pub mod parameter;
pub mod path;
pub mod responses;
pub mod schema;
pub mod types;

pub const DEFINITIONS_REF: &str = "#/definitions/";
pub const RESPONSES_REF: &str = "#/responses/";

pub use items::{Item, Items};
pub use responses::Response;
pub use schema::Schema;
pub use types::Type;

use serde::Deserialize;
use std::marker::PhantomData;

pub(crate) use serde_yaml::Value;

fn trim_reference(ref_: &str) -> &str {
    ref_.trim_start_matches(DEFINITIONS_REF)
        .trim_start_matches(RESPONSES_REF)
}

#[derive(Debug, Deserialize)]
pub struct Swagger<T: Type> {
    pub swagger: String,
    pub definitions: Option<definitions::Definitions>,
    pub paths: Option<path::Paths>,
    pub responses: Option<responses::Responses>,
    #[serde(skip_deserializing)]
    _data: PhantomData<T>,
}

impl<T: Type> Swagger<T> {
    pub fn get_ref_schema(&self, ref_: &str) -> Option<&Schema> {
        log::debug!("getting schema for reference `{ref_}`");
        if ref_.starts_with(DEFINITIONS_REF) {
            if let Some(definitions) = &self.definitions {
                return definitions.get(ref_);
            }
        } else if ref_.starts_with(RESPONSES_REF) {
            if let Some(responses) = &self.responses {
                let response = responses.0.get(ref_)?;
                match response {
                    Response::Object(response) => return response.schema.as_ref(),
                    Response::Reference(ref_) => return self.get_ref_schema(&ref_),
                }
            }
        }

        None
    }

    pub fn merge_all_of_schema(&self, schema: Schema) -> Schema {
        if !schema.all_of.is_empty() {
            let base_schema = Schema {
                description: schema.description.clone(),
                title: schema.title.clone(),
                properties: Some(Items::default()),
                ..Default::default()
            };
            schema
                .all_of
                .into_iter()
                .fold(base_schema, |mut acc, schema| {
                    let mut schema = if let Some(ref_) = &schema.ref_ {
                        self.get_ref_schema(ref_)
                            .map(|s| s.clone())
                            .unwrap_or(schema)
                    } else {
                        schema
                    };
                    if let Some(props) = &mut acc.properties {
                        if let Some(new_props) = &schema.properties {
                            props
                                .0
                                .extend(new_props.0.iter().map(|(k, v)| (k.clone(), v.clone())));
                        }
                    }
                    macro_rules! add_if_not_set {
                    ($($field:ident),+) => {
                        $(
                        if acc.$field.is_none() && schema.$field.is_some() {
                            acc.$field = schema.$field;
                        }
                        )+
                    };
                }
                    add_if_not_set!(format, title, description, type_);

                    if acc.required.is_empty() && !schema.required.is_empty() {
                        acc.required.append(&mut schema.required);
                    }

                    if acc.enum_.is_empty() && !schema.enum_.is_empty() {
                        acc.enum_.append(&mut schema.enum_);
                    }

                    acc
                })
        } else {
            schema
        }
    }

    pub fn map_reference_type(
        &self,
        ref_: &str,
        is_required: bool,
        parent_name: Option<&str>,
    ) -> Option<T> {
        T::map_reference_type(ref_, is_required, parent_name, &self)
    }

    pub fn map_item_type(
        &self,
        item: &Item,
        is_required: bool,
        parent_name: Option<&str>,
    ) -> Option<T> {
        T::map_item_type(item, is_required, parent_name, &self)
    }

    pub fn map_schema_type(
        &self,
        schema: &Schema,
        ref_: Option<&str>,
        is_required: bool,
        parent_name: Option<&str>,
    ) -> Option<T> {
        T::map_schema_type(schema, ref_, is_required, parent_name, &self)
    }
}

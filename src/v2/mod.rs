pub mod codegen;
pub mod definitions;
pub mod items;
pub mod operation;
pub mod parameter;
pub mod path;
pub mod responses;
pub mod schema;

pub const DEFINITIONS_REF: &str = "#/definitions/";
pub const RESPONSES_REF: &str = "#/responses/";

use codegen::backend::rust::RustType;
use items::{Item, Items};
use responses::Response;
use schema::Schema;

use log::{debug, trace};
use serde::Deserialize;

pub(crate) use serde_yaml::Value;

fn trim_reference(ref_: &str) -> &str {
    ref_.trim_start_matches(DEFINITIONS_REF)
        .trim_start_matches(RESPONSES_REF)
}

#[derive(Debug, Clone, Deserialize)]
pub struct Swagger {
    pub swagger: String,
    pub definitions: Option<definitions::Definitions>,
    pub paths: Option<path::Paths>,
    pub responses: Option<responses::Responses>,
}

impl Swagger {
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
    ) -> Option<RustType> {
        debug!("mapping reference `{ref_}`, required: {is_required}, parent: {parent_name:?}");
        let schema = self.get_ref_schema(ref_)?;
        trace!("got schema {schema:?}");
        let ref_ = ref_
            .trim_start_matches(RESPONSES_REF)
            .trim_start_matches(DEFINITIONS_REF);
        self.map_schema_type(schema, Some(ref_), is_required, parent_name)
    }

    pub fn map_item_type(
        &self,
        item: &Item,
        is_required: bool,
        parent_name: Option<&str>,
    ) -> Option<RustType> {
        match item {
            Item::Reference(ref_) => self.map_reference_type(ref_, is_required, parent_name),
            Item::Object(item) => self.map_schema_type(item, None, is_required, parent_name),
        }
    }

    pub fn map_schema_type(
        &self,
        schema: &Schema,
        ref_: Option<&str>,
        is_required: bool,
        parent_name: Option<&str>,
    ) -> Option<RustType> {
        let ty = schema.type_()?;
        trace!(
            "mapping schema type, type: {ty}, ref: {ref_:?}, required: {is_required}, parent: {parent_name:?}"
        );
        let mut ty = match ty {
            "integer" => schema
                .format
                .as_ref()
                .and_then(|format| RustType::from_integer_format(format))
                .unwrap_or(RustType::ISize),
            "string" => match schema
                .format
                .as_ref()
                .map(|fmt| fmt.to_lowercase())
                .as_deref()
            {
                Some("date-time") | Some("datetime") | Some("date time") => RustType::DateTime,
                Some("binary") => RustType::Vec(Box::new(RustType::U8)),
                _ => RustType::String,
            },
            "boolean" => RustType::Bool,
            "array" => {
                let ty = if let Some(ref_) = ref_ {
                    RustType::Custom(trim_reference(ref_).to_string())
                } else if let Some(item) = &schema.items {
                    if let Some(ty) = self.map_item_type(item, true, parent_name) {
                        RustType::Vec(Box::new(ty))
                    } else {
                        return None;
                    }
                } else {
                    return None;
                };

                ty
            }
            "object" => {
                let ty = if let Some(ref_) = ref_ {
                    RustType::Custom(trim_reference(ref_).to_string())
                } else if let Some(item) = &schema.additional_properties {
                    if let Some(ty) = self.map_item_type(item, true, parent_name) {
                        RustType::Object(Box::new(ty))
                    } else {
                        return None;
                    }
                } else if let Some(item) = &schema.items {
                    if let Some(ty) = self.map_item_type(item, true, parent_name) {
                        RustType::Object(Box::new(ty))
                    } else {
                        return None;
                    }
                } else if schema.properties.is_some() {
                    if let Some(name) = schema.name() {
                        RustType::Custom(name)
                    } else if let Some(parent_name) = &parent_name {
                        RustType::Custom(format!("{parent_name}InlineItem"))
                    } else {
                        RustType::Value
                    }
                } else {
                    RustType::Value
                };

                ty
            }
            "number" => {
                let ty = match schema.format.as_deref() {
                    Some("double") => RustType::F64,
                    Some("float") => RustType::F32,
                    _ => return None,
                };
                ty
            }
            _ => return None,
        };
        if !is_required {
            ty = RustType::Option(Box::new(ty));
        }
        trace!("mapped to {ty}");
        Some(ty)
    }
}

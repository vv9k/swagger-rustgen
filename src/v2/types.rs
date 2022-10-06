use crate::v2::{schema::Schema, Item, Swagger, DEFINITIONS_REF, RESPONSES_REF};
use log::{debug, trace};

pub trait Type: std::fmt::Display + Sized {
    fn format_name(name: &str) -> String;

    fn map_schema_type(
        schema: &Schema,
        ref_: Option<&str>,
        is_required: bool,
        parent_name: Option<&str>,
        swagger: &Swagger<Self>,
    ) -> Option<Self>;

    fn map_item_type(
        item: &Item,
        is_required: bool,
        parent_name: Option<&str>,
        swagger: &Swagger<Self>,
    ) -> Option<Self> {
        match item {
            Item::Reference(ref_) => {
                Self::map_reference_type(ref_, is_required, parent_name, swagger)
            }
            Item::Object(item) => {
                Self::map_schema_type(item, None, is_required, parent_name, swagger)
            }
        }
    }

    fn map_reference_type(
        ref_: &str,
        is_required: bool,
        parent_name: Option<&str>,
        swagger: &Swagger<Self>,
    ) -> Option<Self> {
        debug!("mapping reference `{ref_}`, required: {is_required}, parent: {parent_name:?}");
        let schema = swagger.get_ref_schema(ref_)?;
        trace!("got schema {schema:?}");
        let ref_ = ref_
            .trim_start_matches(RESPONSES_REF)
            .trim_start_matches(DEFINITIONS_REF);
        Self::map_schema_type(schema, Some(ref_), is_required, parent_name, swagger)
    }
}

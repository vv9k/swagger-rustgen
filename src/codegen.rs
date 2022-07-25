use crate::{
    items::{Item, ItemsObject},
    responses::Response,
    schema::Schema,
    types::RustType,
    Swagger, DEFINITIONS_REF, RESPONSES_REF,
};

pub struct CodeGenerator {
    swagger: Swagger,
}

impl CodeGenerator {
    pub fn new(swagger: Swagger) -> Self {
        Self { swagger }
    }

    pub fn generate_models(&self, mut writer: &mut impl std::io::Write) -> std::io::Result<()> {
        write!(writer, "/// DEFINITIONS ///\n\n\n")?;
        if let Some(definitions) = &self.swagger.definitions {
            let mut definitions: Vec<_> = definitions.0.iter().collect();
            definitions.sort_unstable_by_key(|(k, _)| *k);

            for (name, schema) in definitions {
                self.handle_schema(name, schema, &mut writer)?;
            }
        };

        write!(writer, "\n\n\n/// RESPONSES ///\n\n\n")?;
        if let Some(responses) = &self.swagger.responses {
            let mut responses: Vec<_> = responses.0.iter().collect();
            responses.sort_unstable_by_key(|(k, _)| *k);

            for (name, response) in responses {
                match response {
                    Response::Reference(ref_) => {
                        if let Some(ty) = self.map_reference(ref_) {
                            writeln!(writer, "pub type {name} = {ty};\n")?;
                        }
                    }
                    Response::Object(response) => {
                        self.print_doc_comment(&response.description, None, &mut writer)?;
                        if let Some(schema) = &response.schema {
                            self.handle_schema(name, schema, &mut writer)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn map_reference(&self, ref_: &str) -> Option<RustType> {
        if ref_.starts_with(DEFINITIONS_REF) {
            if let Some(definitions) = &self.swagger.definitions {
                let schema = definitions.get(ref_)?;
                schema.map_type(Some(ref_))
            } else {
                None
            }
        } else if ref_.starts_with(RESPONSES_REF) {
            if let Some(responses) = &self.swagger.responses {
                let response = responses.0.get(ref_)?;
                match response {
                    Response::Object(response) => response.schema.as_ref()?.map_type(Some(ref_)),
                    Response::Reference(ref_) => self.map_reference(ref_),
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    fn map_item_object(&self, item: &ItemsObject, is_required: bool) -> Option<RustType> {
        let ty = item.type_.as_deref()?;
        let mut ty = match ty {
            "integer" => item
                .format
                .as_ref()
                .and_then(|format| RustType::from_integer_format(format))
                .unwrap_or(RustType::USize),
            "string" => RustType::String,
            "boolean" => RustType::Bool,
            "array" => match &item.items {
                Some(Item::Reference(ref_)) => {
                    let ty = self.map_reference(ref_)?;
                    RustType::Vec(Box::new(ty))
                }
                Some(item) => {
                    let ty = self.map_item_type(item, true)?;
                    RustType::Vec(Box::new(ty))
                }
                None => return None,
            },
            "object" => match &item.additional_properties {
                Some(Item::Reference(ref_)) => {
                    let ty = self.map_reference(ref_)?;
                    RustType::Object(Box::new(ty))
                }
                Some(item) => {
                    let ty = self.map_item_type(item, true)?;
                    RustType::Object(Box::new(ty))
                }
                None => RustType::Object(Box::new(RustType::JsonValue)),
            },
            "number" => match item.format.as_deref() {
                Some("double") => RustType::F64,
                Some("float") => RustType::F32,
                _ => return None,
            },
            _ => return None,
        };

        if !is_required {
            ty = RustType::Option(Box::new(ty));
        }

        Some(ty)
    }

    fn map_item_type(&self, item: &Item, is_required: bool) -> Option<RustType> {
        match item {
            Item::Reference(ref_) => self.map_reference(ref_),
            Item::Object(item) => self.map_item_object(item, is_required),
        }
    }

    fn print_doc_comment(
        &self,
        comment: impl AsRef<str>,
        indentation: Option<u8>,
        writer: &mut impl std::io::Write,
    ) -> std::io::Result<()> {
        let indentation = indentation
            .map(|i| " ".repeat(i.into()))
            .unwrap_or_default();
        for line in comment.as_ref().lines() {
            writeln!(writer, "{indentation}/// {line}")?;
        }
        Ok(())
    }

    fn handle_schema(
        &self,
        name: &str,
        schema: &Schema,
        writer: &mut impl std::io::Write,
    ) -> std::io::Result<()> {
        if let Some(description) = &schema.description {
            self.print_doc_comment(description, None, writer)?;
        }

        if let Some(props) = &schema.properties {
            writeln!(writer, "pub struct {name} {{")?;
            let required = schema.required.clone().unwrap_or_default();
            for (prop, item) in &props.0 {
                match item {
                    Item::Reference(ref_) => {
                        if let Some(ty) = self.map_reference(ref_) {
                            writeln!(writer, "    {prop}: {ty},")?;
                        }
                    }
                    it @ Item::Object(item) => {
                        if let Some(descr) = &item.description {
                            self.print_doc_comment(descr, Some(4), writer)?;
                        }
                        write!(writer, "    {prop}: ")?;

                        let is_required = required.contains(prop);

                        if let Some(ty) = self.map_item_type(it, is_required) {
                            write!(writer, "{ty}")?;
                        } else if item.type_.as_deref() == Some("object") {
                        }
                        writeln!(writer, ",")?;
                    }
                }
            }
            writeln!(writer, "}}\n")?;
        } else if let Some(ty) = schema.map_type(None) {
            writeln!(writer, "pub type {name} = {ty};\n")?;
        } else if let Some(ref_) = schema.ref_.as_deref() {
            let _ty = self.map_reference(ref_);
        }

        Ok(())
    }
}

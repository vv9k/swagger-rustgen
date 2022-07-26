use crate::v2::{
    items::{Item, ItemsObject},
    path::Path,
    responses::Response,
    schema::Schema,
    trim_reference, Swagger, DEFINITIONS_REF, RESPONSES_REF,
};
use crate::{
    name::{format_type_name, format_var_name},
    types::RustType,
};

pub struct CodeGenerator {
    swagger: Swagger,
    processed_types: Vec<String>,
}

impl CodeGenerator {
    pub fn new(swagger: Swagger) -> Self {
        Self {
            swagger,
            processed_types: vec![],
        }
    }

    pub fn generate_models(&mut self, mut writer: &mut impl std::io::Write) -> std::io::Result<()> {
        let swagger = self.swagger.clone();
        self.generate_definitions_models(&swagger, &mut writer)?;
        self.generate_responses_models(&swagger, &mut writer)?;
        self.generate_inline_responses_models(&swagger, &mut writer)
    }

    fn generate_definitions_models(
        &mut self,
        swagger: &Swagger,
        writer: &mut impl std::io::Write,
    ) -> std::io::Result<()> {
        if let Some(definitions) = &swagger.definitions {
            let mut definitions: Vec<_> = definitions.0.iter().collect();
            definitions.sort_unstable_by_key(|(k, _)| *k);

            writeln!(writer, "// DEFINITIONS\n")?;

            for (name, schema) in definitions {
                self.handle_schema(name, schema, writer)?;
            }
        };
        Ok(())
    }

    fn generate_responses_models(
        &mut self,
        swagger: &Swagger,
        writer: &mut impl std::io::Write,
    ) -> std::io::Result<()> {
        if let Some(responses) = &swagger.responses {
            let mut responses: Vec<_> = responses.0.iter().collect();
            responses.sort_unstable_by_key(|(k, _)| *k);

            writeln!(writer, "\n\n// RESPONSES\n")?;

            for (name, response) in responses {
                match response {
                    Response::Reference(ref_) => {
                        if let Some(ty) = self.map_reference(ref_) {
                            let type_name = format_type_name(name);
                            if self.processed_types.contains(&type_name) {
                                continue;
                            } else {
                                self.processed_types.push(type_name.clone());
                            }

                            writeln!(writer, "pub type {type_name} = {};\n", ty.to_string())?;
                        }
                    }
                    Response::Object(response) => {
                        if let Some(schema) = &response.schema {
                            let mut schema = schema.clone();
                            schema.description = response.description.clone();
                            self.handle_schema(name, &schema, writer)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn generate_inline_responses_models(
        &mut self,
        swagger: &Swagger,
        writer: &mut impl std::io::Write,
    ) -> std::io::Result<()> {
        if let Some(paths) = &swagger.paths {
            let mut paths: Vec<_> = paths.0.iter().collect();
            paths.sort_unstable_by_key(|(k, _)| *k);

            writeln!(writer, "\n\n// INLINE RESPONSES\n")?;

            macro_rules! handle_method {
                ($path:ident, $method:ident) => {
                    if $path.$method.is_none() {
                        continue;
                    }
                    let op = $path.$method.as_ref().unwrap();
                    for (code, response) in &op.responses.0 {
                        match response {
                            Response::Object(response) => {
                                if let Some(schema) = &response.schema {
                                    let mut schema = schema.clone();
                                    schema.description = response.description.clone();
                                    self.handle_schema(
                                        &format!(
                                            "{}{code}Response",
                                            op.operation_id.as_deref().unwrap_or("InlineResponse")
                                        ),
                                        &schema,
                                        writer,
                                    )?;
                                }
                            }
                            _ => {}
                        }
                    }
                };
            }

            for (_name, path) in paths {
                match path {
                    Path::Item(path) => {
                        handle_method!(path, get);
                        handle_method!(path, put);
                        handle_method!(path, post);
                        handle_method!(path, delete);
                        handle_method!(path, options);
                        handle_method!(path, head);
                        handle_method!(path, patch);
                    }
                    Path::Extension(ext) => eprintln!("{:?}", ext),
                }
            }
        }
        Ok(())
    }

    fn map_reference(&self, ref_: &str) -> Option<RustType> {
        if ref_.starts_with(DEFINITIONS_REF) {
            if let Some(definitions) = &self.swagger.definitions {
                let schema = definitions.get(ref_)?;
                self.map_schema_type(schema, Some(ref_))
            } else {
                None
            }
        } else if ref_.starts_with(RESPONSES_REF) {
            if let Some(responses) = &self.swagger.responses {
                let response = responses.0.get(ref_)?;
                match response {
                    Response::Object(response) => {
                        self.map_schema_type(response.schema.as_ref()?, Some(ref_))
                    }
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
            "string" => match item.format.as_deref() {
                Some("date-time") => RustType::DateTime,
                Some("binary") => RustType::Vec(Box::new(RustType::U8)),
                _ => RustType::String,
            },
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
                None => RustType::Object(Box::new(RustType::Value)),
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

    fn map_schema_type(&self, schema: &Schema, ref_: Option<&str>) -> Option<RustType> {
        let ty = schema.type_.as_deref()?;
        match ty {
            "integer" => schema
                .format
                .as_ref()
                .and_then(|format| RustType::from_integer_format(format))
                .or(Some(RustType::USize)),
            "string" => match schema.format.as_deref() {
                Some("date-time") => Some(RustType::DateTime),
                Some("binary") => Some(RustType::Vec(Box::new(RustType::U8))),
                _ => Some(RustType::String),
            },
            "boolean" => Some(RustType::Bool),
            "array" => {
                if let Some(ref_) = ref_ {
                    return Some(RustType::Vec(Box::new(RustType::Custom(
                        trim_reference(ref_).to_string(),
                    ))));
                } else if let Some(item) = &schema.items {
                    if let Some(ty) = self.map_item_type(item, true) {
                        return Some(RustType::Vec(Box::new(ty)));
                    }
                }
                None
            }
            "object" => {
                if let Some(ref_) = ref_ {
                    return Some(RustType::Custom(trim_reference(ref_).to_string()));
                } else if let Some(item) = &schema.additional_properties {
                    if let Some(ty) = self.map_item_type(item, true) {
                        return Some(RustType::Object(Box::new(RustType::Option(Box::new(ty)))));
                    }
                } else if let Some(item) = &schema.items {
                    if let Some(ty) = self.map_item_type(item, true) {
                        return Some(RustType::Object(Box::new(ty)));
                    }
                } else {
                    return Some(RustType::Value);
                }
                None
            }
            "number" => {
                let ty = match schema.format.as_deref() {
                    Some("double") => RustType::F64,
                    Some("float") => RustType::F32,
                    _ => return None,
                };
                Some(ty)
            }
            _ => None,
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
        &mut self,
        name: &str,
        schema: &Schema,
        writer: &mut impl std::io::Write,
    ) -> std::io::Result<()> {
        if let Some(props) = &schema.properties {
            let type_name = format_type_name(name);

            if self.processed_types.contains(&type_name) {
                return Ok(());
            } else {
                self.processed_types.push(type_name.clone());
            }

            writeln!(
                writer,
                "#[derive(Debug, Clone, Deserialize, PartialEq, Serialize, Deserialize)]"
            )?;
            if let Some(description) = &schema.description {
                self.print_doc_comment(description, None, writer)?;
            }

            writeln!(writer, "pub struct {} {{", type_name)?;
            let required = schema.required.clone().unwrap_or_default();
            let mut props: Vec<_> = props.0.iter().collect();
            props.sort_unstable_by_key(|(k, _)| *k);
            for (prop, item) in props {
                match item {
                    Item::Reference(ref_) => {
                        let ty = if let Some(ty) = self.map_reference(ref_) {
                            ty
                        } else {
                            RustType::Option(Box::new(RustType::Value))
                        };
                        let formatted_var = format_var_name(prop);
                        if &formatted_var != prop {
                            writeln!(writer, "    #[serde(rename = \"{prop}\")]")?;
                        }
                        writeln!(writer, "    {formatted_var}: {ty},")?;
                    }
                    it @ Item::Object(item) => {
                        let formatted_var = format_var_name(prop);
                        let is_required = required.contains(prop);

                        let ty = if let Some(ty) = self.map_item_type(it, is_required) {
                            ty
                        } else {
                            RustType::Option(Box::new(RustType::Value))
                        };

                        if &formatted_var != prop {
                            writeln!(writer, "    #[serde(rename = \"{prop}\")]")?;
                        }

                        if matches!(ty, RustType::Vec(_) | RustType::Object(_)) {
                            writeln!(writer, "    #[serde(default)]")?;
                        }

                        if !is_required {
                            writeln!(
                                writer,
                                "    #[serde(skip_serializing_if = \"Option::is_none\")]"
                            )?;
                        }

                        if let Some(descr) = &item.description {
                            self.print_doc_comment(descr, Some(4), writer)?;
                        }

                        writeln!(writer, "    {formatted_var}: {ty},")?;
                    }
                }
            }
            writeln!(writer, "}}\n")?;
        } else if let Some(ty) = self.map_schema_type(schema, None) {
            let type_name = format_type_name(name);

            if self.processed_types.contains(&type_name) {
                return Ok(());
            } else {
                self.processed_types.push(type_name.clone());
            }

            if let Some(description) = &schema.description {
                self.print_doc_comment(description, None, writer)?;
            }

            writeln!(writer, "pub type {type_name} = {};\n", ty.to_string())?;
        } else if let Some(ref_) = schema.ref_.as_deref() {
            let _ty = self.map_reference(ref_);
            //eprintln!("else if {}", _ty.unwrap_or(RustType::Bool));
        } else {
            //eprintln!("else {:?}", schema);
        }

        Ok(())
    }
}

use crate::v2::{
    items::{Item, Items},
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

            for (name, response) in responses {
                match response {
                    Response::Reference(ref_) => {
                        if let Some(ty) = self.map_reference(ref_, true, Some(name)) {
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

            macro_rules! handle_method {
                ($path:ident, $method:ident) => {
                    if let Some(op) = $path.$method.as_ref() {
                        for (code, response) in &op.responses.0 {
                            match response {
                                Response::Object(response) => {
                                    if let Some(schema) = &response.schema {
                                        let mut schema = schema.clone();
                                        schema.description = response.description.clone();
                                        self.handle_schema(
                                            &format!(
                                                "{}{code}Response",
                                                op.operation_id
                                                    .as_deref()
                                                    .unwrap_or("InlineResponse")
                                            ),
                                            &schema,
                                            writer,
                                        )?;
                                    }
                                }
                                _ => {}
                            }
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

    fn map_reference(
        &self,
        ref_: &str,
        is_required: bool,
        parent_name: Option<&str>,
    ) -> Option<RustType> {
        if ref_.starts_with(DEFINITIONS_REF) {
            if let Some(definitions) = &self.swagger.definitions {
                let schema = definitions.get(ref_)?;
                self.map_schema_type(schema, Some(ref_), is_required, parent_name)
            } else {
                None
            }
        } else if ref_.starts_with(RESPONSES_REF) {
            if let Some(responses) = &self.swagger.responses {
                let response = responses.0.get(ref_)?;
                match response {
                    Response::Object(response) => self.map_schema_type(
                        response.schema.as_ref()?,
                        Some(ref_),
                        is_required,
                        parent_name,
                    ),
                    Response::Reference(ref_) => self.map_reference(ref_, is_required, parent_name),
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    fn map_item_type(
        &self,
        item: &Item,
        is_required: bool,
        parent_name: Option<&str>,
    ) -> Option<RustType> {
        match item {
            Item::Reference(ref_) => self.map_reference(ref_, is_required, parent_name),
            Item::Object(item) => self.map_schema_type(item, None, is_required, parent_name),
        }
    }

    fn map_schema_type(
        &self,
        schema: &Schema,
        ref_: Option<&str>,
        is_required: bool,
        parent_name: Option<&str>,
    ) -> Option<RustType> {
        let ty = schema.type_.as_deref()?;
        let mut ty = match ty {
            "integer" => schema
                .format
                .as_ref()
                .and_then(|format| RustType::from_integer_format(format))
                .unwrap_or(RustType::USize),
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
                    RustType::Vec(Box::new(RustType::Custom(trim_reference(ref_).to_string())))
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
                        RustType::Object(Box::new(RustType::Option(Box::new(ty))))
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
                    if let Some(title) = &schema.x_go_name {
                        RustType::Custom(title.to_string())
                    } else if let Some(title) = &schema.title {
                        RustType::Custom(title.to_string())
                    } else if let Some(title) = parent_name {
                        RustType::Custom(title.to_string())
                    } else {
                        RustType::Value
                    }
                } else {
                    eprintln!("else value {:#?}", schema);
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
        Some(ty)
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
        let type_name = format_type_name(name);
        if let Some(props) = &schema.properties {
            if self.processed_types.contains(&type_name) {
                return Ok(());
            } else {
                self.processed_types.push(type_name.clone());
            }

            writeln!(
                writer,
                "#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]"
            )?;
            if let Some(description) = &schema.description {
                self.print_doc_comment(description, None, writer)?;
            }

            writeln!(writer, "pub struct {} {{", type_name)?;
            let required = schema.required.clone().unwrap_or_default();
            let mut props: Vec<_> = props.0.iter().collect();
            props.sort_unstable_by_key(|(k, _)| *k);
            for (prop, item) in &props {
                let is_required = required.contains(prop);
                match item {
                    Item::Reference(ref_) => {
                        let ty = if let Some(ty) = self.map_reference(ref_, is_required, Some(prop))
                        {
                            ty
                        } else {
                            RustType::Option(Box::new(RustType::Value))
                        };
                        let formatted_var = format_var_name(prop);
                        if &&formatted_var != prop {
                            writeln!(writer, "    #[serde(rename = \"{prop}\")]")?;
                        }
                        writeln!(writer, "    pub {formatted_var}: {ty},")?;
                    }
                    it @ Item::Object(item) => {
                        let formatted_var = format_var_name(prop);

                        let ty = if let Some(ty) = self.map_item_type(it, is_required, Some(prop)) {
                            ty
                        } else {
                            RustType::Option(Box::new(RustType::Value))
                        };

                        if &&formatted_var != prop {
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

                        writeln!(writer, "    pub {formatted_var}: {ty},")?;
                    }
                }
            }
            writeln!(writer, "}}\n")?;

            //for (prop, schema) in props {
            //match schema {
            //Item::Object(schema) if schema.properties.is_some() => {
            //self.handle_schema(prop, schema, writer)?;
            //}
            //_ => {}
            //}
            //}
        } else if let Some(ty) = self.map_schema_type(schema, None, true, Some(name)) {
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
            let _ty = self.map_reference(ref_, true, Some(name));
            //eprintln!("else if {}", _ty.unwrap_or(RustType::Bool));
        } else if let Some(all_of) = &schema.all_of {
            let base_schema = Schema {
                description: schema.description.clone(),
                title: schema.title.clone(),
                properties: Some(Items::default()),
                ..Default::default()
            };
            let schema = all_of.into_iter().fold(base_schema, |mut acc, schema| {
                if let Some(props) = &mut acc.properties {
                    if let Some(new_props) = &schema.properties {
                        props
                            .0
                            .extend(new_props.0.iter().map(|(k, v)| (k.clone(), v.clone())));
                    }
                }
                acc
            });
            return self.handle_schema(name, &schema, writer);
        } else {
            eprintln!("else {:?}", schema);
        }

        Ok(())
    }
}

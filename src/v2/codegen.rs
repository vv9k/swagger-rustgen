use crate::v2::{
    items::Item, path::Path, responses::Response, schema::Schema, trim_reference, Swagger,
    DEFINITIONS_REF, RESPONSES_REF,
};
use crate::{
    name::{format_type_name, format_var_name},
    types::RustType,
};

use std::cmp::Ordering;

#[derive(Debug)]
struct ModelPrototype {
    pub name: String,
    pub parent_name: Option<String>,
    pub schema: Item,
}

pub struct CodeGenerator {
    swagger: Swagger,
    processed_types: Vec<String>,
    models_to_generate: Vec<ModelPrototype>,
}

impl CodeGenerator {
    pub fn new(swagger: Swagger) -> Self {
        Self {
            swagger,
            processed_types: vec![],
            models_to_generate: vec![],
        }
    }

    pub fn generate_models(&mut self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        let swagger = self.swagger.clone();
        self.add_definition_models(&swagger);
        self.add_responses_models(&swagger);
        self.add_paths_models(&swagger);
        self.generate(writer)?;

        Ok(())
    }

    fn generate(&mut self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        let mut models = std::mem::take(&mut self.models_to_generate);
        models.sort_by(
            |a, b| match (a.schema.is_reference(), b.schema.is_reference()) {
                (true, true) | (false, false) => Ordering::Equal,
                (true, false) => Ordering::Greater,
                (false, true) => Ordering::Less,
            },
        );
        for model in models {
            match model.schema {
                Item::Reference(ref_) => {
                    let should_process = self
                        .get_ref_schema(&ref_)
                        .map(|s| s.is_object())
                        .unwrap_or_default();
                    if should_process {
                        if let Some(ty) = self.map_reference(&ref_, true, Some(&model.name)) {
                            let type_name = format_type_name(&model.name);
                            if self.processed_types.contains(&type_name) {
                                continue;
                            } else {
                                self.processed_types.push(type_name.clone());
                            }
                            writeln!(writer, "pub type {type_name} = {};\n", ty.to_string())?;
                        }
                    }
                }
                Item::Object(schema) => {
                    let schema = schema.merge_all_of_schema();
                    self.handle_schema(&model.name, model.parent_name.as_deref(), &schema, writer)?;
                }
            }
        }
        Ok(())
    }

    fn add_ref_prototype(
        &mut self,
        name: impl Into<String>,
        parent_name: Option<String>,
        ref_: String,
    ) {
        self.models_to_generate.push(ModelPrototype {
            name: name.into(),
            parent_name,
            schema: Item::Reference(ref_),
        });
    }

    fn add_schema_prototype(
        &mut self,
        name: impl Into<String>,
        parent_name: Option<String>,
        schema: &Schema,
    ) {
        let name = name.into();
        if let Some(ref_) = &schema.ref_ {
            self.add_ref_prototype(name, parent_name, ref_.to_string());
            return;
        }

        if let Some(items) = &schema.items {
            match items {
                Item::Object(child_schema) => {
                    if child_schema.is_object() {
                        self.add_schema_prototype("", Some(name.clone()), &child_schema)
                    }
                }
                _ => {}
            }
        }

        if let Some(props) = &schema.properties {
            for (prop_name, prop_schema) in props.0.iter() {
                match prop_schema {
                    Item::Object(prop_schema) => {
                        if prop_schema.is_object() && prop_schema.properties.is_some() {
                            let prop_name = format!("{name}{prop_name}InlineItem");
                            self.add_schema_prototype(prop_name, Some(name.clone()), &prop_schema)
                        }
                    }
                    _ => {}
                }
            }
        }

        self.models_to_generate.push(ModelPrototype {
            name: name.into(),
            parent_name,
            schema: Item::Object(Box::new(schema.clone())),
        });
    }

    fn add_definition_models(&mut self, swagger: &Swagger) {
        if let Some(definitions) = &swagger.definitions {
            let mut definitions: Vec<_> = definitions.0.iter().collect();
            definitions.sort_unstable_by_key(|(k, _)| *k);

            for (name, schema) in definitions {
                self.add_schema_prototype(name, None, schema);
            }
        };
    }

    fn add_responses_models(&mut self, swagger: &Swagger) {
        if let Some(responses) = &swagger.responses {
            let mut responses: Vec<_> = responses.0.iter().collect();
            responses.sort_unstable_by_key(|(k, _)| *k);

            for (name, response) in responses {
                match response {
                    Response::Object(response) => {
                        if let Some(schema) = &response.schema {
                            let mut schema = schema.clone();
                            schema.description = response.description.clone();
                            self.add_schema_prototype(name, None, &schema);
                        }
                    }
                    Response::Reference(ref_) => {
                        self.add_ref_prototype(name, None, ref_.to_string())
                    }
                }
            }
        }
    }

    fn add_paths_models(&mut self, swagger: &Swagger) {
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
                                        self.add_schema_prototype(
                                            &format!(
                                                "{}{code}Response",
                                                op.operation_id
                                                    .as_deref()
                                                    .unwrap_or("InlineResponse")
                                            ),
                                            None,
                                            &schema,
                                        );
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
    }

    fn get_ref_schema(&self, ref_: &str) -> Option<&Schema> {
        if ref_.starts_with(DEFINITIONS_REF) {
            if let Some(definitions) = &self.swagger.definitions {
                return definitions.get(ref_);
            }
        } else if ref_.starts_with(RESPONSES_REF) {
            if let Some(responses) = &self.swagger.responses {
                let response = responses.0.get(ref_)?;
                match response {
                    Response::Object(response) => return response.schema.as_ref(),
                    Response::Reference(ref_) => return self.get_ref_schema(ref_),
                }
            }
        }

        None
    }

    fn map_reference(
        &self,
        ref_: &str,
        is_required: bool,
        parent_name: Option<&str>,
    ) -> Option<RustType> {
        let schema = self.get_ref_schema(ref_)?;
        let ref_ = ref_
            .trim_start_matches(RESPONSES_REF)
            .trim_start_matches(DEFINITIONS_REF);
        self.map_schema_type(schema, Some(ref_), is_required, parent_name)
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
        let ty = schema.type_()?;
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
                    if let Some(title) = &schema.x_go_name {
                        RustType::Custom(title.to_string())
                    } else if let Some(title) = &schema.title {
                        RustType::Custom(title.to_string())
                    } else if let Some(title) = parent_name {
                        RustType::Custom(format!("{title}InlineItem"))
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
        parent_name: Option<&str>,
        schema: &Schema,
        writer: &mut impl std::io::Write,
    ) -> std::io::Result<()> {
        let name = if name.is_empty() {
            if let Some(title) = &schema.title {
                title.into()
            } else if let Some(parent_name) = parent_name {
                format!("{parent_name}InlineItem")
            } else {
                Default::default()
            }
        } else {
            name.into()
        };
        let mut type_name = format_type_name(&name);
        if let Some(props) = &schema.properties {
            if self.processed_types.contains(&type_name) {
                type_name.push_str("_");
            }

            self.processed_types.push(type_name.clone());

            self.print_derives(&schema, writer)?;
            self.print_description(&schema, writer)?;

            writeln!(writer, "pub struct {} {{", type_name)?;
            let mut props: Vec<_> = props.0.iter().collect();
            props.sort_unstable_by_key(|(k, _)| *k);
            for (prop, item) in &props {
                let is_required = schema
                    .required
                    .as_ref()
                    .map(|r| r.contains(prop))
                    .unwrap_or_default();
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

                        let prop_ty_name = if item.is_object() {
                            format!("{type_name}{prop}")
                        } else {
                            prop.to_string()
                        };

                        let ty = if let Some(ty) =
                            self.map_item_type(it, is_required, Some(&prop_ty_name))
                        {
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
        } else if schema.is_array() {
            self.handle_array_schema(&name, schema, writer)?
        } else if let Some(ref_) = schema.ref_.as_deref() {
            let _ty = self.map_reference(ref_, true, Some(&name));
        } else if let Some(ty) = self.map_schema_type(schema, None, true, Some(&name)) {
            if self.was_processed(&type_name) {
                return Ok(());
            } else {
                self.processed_types.push(type_name.clone());
            }

            if let Some(description) = &schema.description {
                self.print_doc_comment(description, None, writer)?;
            }

            writeln!(writer, "pub type {type_name} = {};\n", ty.to_string())?;
        } else {
            eprintln!("else {:?}", schema);
        }

        Ok(())
    }

    fn handle_array_schema(
        &mut self,
        name: &str,
        schema: &Schema,
        writer: &mut impl std::io::Write,
    ) -> std::io::Result<()> {
        if let Some(item) = &schema.items {
            let ty = self.map_item_type(&item, true, Some(&name));
            if ty.is_none() {
                return Ok(());
            }
            let ty = ty.unwrap();
            //let item_type_name = format_type_name(ty.to_string().as_str());
            let ty = RustType::Vec(Box::new(ty));
            self.print_description(&schema, writer)?;
            writeln!(writer, "pub type {} = {ty};\n", format_type_name(name))?;
        }
        Ok(())
    }

    fn print_derives(
        &self,
        _schema: &Schema,
        writer: &mut impl std::io::Write,
    ) -> std::io::Result<()> {
        const DEFAULT_DERIVES: &str = "#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]";
        writeln!(writer, "{DEFAULT_DERIVES}")
    }

    fn print_description(
        &self,
        schema: &Schema,
        writer: &mut impl std::io::Write,
    ) -> std::io::Result<()> {
        if let Some(description) = &schema.description {
            self.print_doc_comment(description, None, writer)?;
        }
        Ok(())
    }

    fn was_processed(&mut self, type_: impl AsRef<str>) -> bool {
        let type_ = type_.as_ref();
        let res = self.processed_types.iter().any(|ty| ty == type_);
        res
    }
}

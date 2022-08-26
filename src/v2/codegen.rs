use crate::v2::{
    items::Item, parameter::Parameter, path::Path, responses::Response, schema::Schema,
    trim_reference, Swagger, DEFINITIONS_REF, RESPONSES_REF,
};
use crate::{
    name::{format_enum_value_name, format_type_name, format_var_name},
    types::RustType,
};

use log::{debug, error, trace};
use std::cmp::Ordering;

#[derive(Debug)]
struct ModelPrototype {
    pub name: String,
    pub parent_name: Option<String>,
    pub schema: Item,
}

pub struct CodeGenerator {
    swagger: Swagger,
    models_to_generate: Vec<ModelPrototype>,
}

impl CodeGenerator {
    pub fn new(swagger: Swagger) -> Self {
        Self {
            swagger,
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

        // Generate object schemas first so that all references are valid
        // and fallback to alphabetical sorting
        models.sort_by(
            |a, b| match (a.schema.is_reference(), b.schema.is_reference()) {
                (true, true) | (false, false) => a.name.cmp(&b.name),
                (true, false) => Ordering::Greater,
                (false, true) => Ordering::Less,
            },
        );

        for model in models {
            trace!("generating {} `{}`", model.schema.type_(), &model.name);
            match model.schema {
                Item::Reference(ref_) => {
                    if let Some(schema) = self.get_ref_schema(&ref_) {
                        if !schema.is_object() {
                            continue;
                        }
                        if let Some(ty) = self.map_reference(&ref_, true, Some(&model.name)) {
                            let type_name = format_type_name(&model.name);
                            let ty_str = ty.to_string();

                            if type_name == ty_str {
                                log::warn!(
                                    "skipping type alias with same name `{type_name} == {ty_str}`"
                                );
                                continue;
                            }
                            self.print_description(&schema, writer)?;
                            writeln!(writer, "pub type {type_name} = {ty_str};\n")?;
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
        let prototype = ModelPrototype {
            name: name.into(),
            parent_name,
            schema: Item::Reference(ref_),
        };
        trace!("adding reference {prototype:?}");
        self.models_to_generate.push(prototype);
    }

    fn add_schema_prototype(
        &mut self,
        name: impl Into<String>,
        parent_name: Option<String>,
        schema: &Schema,
    ) {
        let mut name = name.into();
        if name.ends_with("InlineItem") {
            if let Some(schema_name) = schema.name() {
                name = schema_name;
            }
        }
        trace!("adding schema prototype `{name}`, parent: `{parent_name:?}`");
        if let Some(ref_) = &schema.ref_ {
            self.add_ref_prototype(name, parent_name, ref_.to_string());
            return;
        }

        if let Some(items) = &schema.items {
            match items {
                Item::Object(child_schema) => {
                    if child_schema.is_object() {
                        let name = child_schema.name().unwrap_or(format!("{name}InlineItem"));
                        trace!("handling child schema `{name}` {child_schema:?}");
                        self.add_schema_prototype(name, parent_name.clone(), &child_schema)
                    }
                }
                _ => {}
            }
        }

        if let Some(props) = &schema.properties {
            for (prop_name, prop_schema) in props.0.iter() {
                trace!("handling property {prop_name}, parent: {:?}", &parent_name);
                match prop_schema {
                    Item::Object(prop_schema) => {
                        let prop_name = prop_schema
                            .name()
                            .unwrap_or(format!("{name}{prop_name}InlineItem"));
                        trace!("Item::Object property {prop_name}");
                        if prop_schema.is_object() && prop_schema.properties.is_some() {
                            trace!("adding object schema {prop_name}");
                            self.add_schema_prototype(prop_name, Some(name.clone()), &prop_schema)
                        } else if prop_schema.is_array() {
                            if let Some(items) = &prop_schema.items {
                                trace!("adding array schema {prop_name}");
                                match items {
                                    Item::Object(prop_schema) if prop_schema.is_object() => self
                                        .add_schema_prototype(
                                            prop_name.clone(),
                                            Some(name.clone()),
                                            &prop_schema,
                                        ),
                                    _ => {}
                                }
                            }
                            error!("skipping {prop_name} {prop_schema:?}")
                        } else if prop_schema.is_enum() {
                            trace!("adding enum schema {prop_name}");
                            self.add_schema_prototype(prop_name, Some(name.clone()), &prop_schema)
                        }
                    }
                    _ => {}
                }
            }
        }

        let prototype = ModelPrototype {
            name: name.into(),
            parent_name,
            schema: Item::Object(Box::new(schema.clone())),
        };
        trace!("adding object {prototype:?}");
        self.models_to_generate.push(prototype);
    }

    fn add_definition_models(&mut self, swagger: &Swagger) {
        debug!("adding definition models");
        if let Some(definitions) = &swagger.definitions {
            trace!("definitions found");
            let mut definitions: Vec<_> = definitions.0.iter().collect();
            trace!("sorting definitions alphabetically by name");
            definitions.sort_unstable_by_key(|(k, _)| *k);

            for (name, schema) in definitions {
                trace!("processing definition `{name}`");
                let schema = schema.clone().merge_all_of_schema();
                self.add_schema_prototype(name, None, &schema);
            }
        } else {
            trace!("no definitions to process");
        }
    }

    fn add_responses_models(&mut self, swagger: &Swagger) {
        debug!("adding responses models");
        if let Some(responses) = &swagger.responses {
            trace!("responses found");
            let mut responses: Vec<_> = responses.0.iter().collect();
            trace!("sorting responses alphabetically by name");
            responses.sort_unstable_by_key(|(k, _)| *k);

            for (name, response) in responses {
                trace!("processing response `{name}`");
                match response {
                    Response::Object(response) => {
                        if let Some(schema) = &response.schema {
                            let mut schema = schema.clone();
                            schema.description = response.description.clone();
                            let schema = schema.merge_all_of_schema();
                            self.add_schema_prototype(name, None, &schema);
                        }
                    }
                    Response::Reference(ref_) => {
                        self.add_ref_prototype(name, None, ref_.to_string())
                    }
                }
            }
        } else {
            trace!("no responses to process");
        }
    }

    fn add_paths_models(&mut self, swagger: &Swagger) {
        debug!("adding paths models");
        if let Some(paths) = &swagger.paths {
            debug!("paths found");
            let mut paths: Vec<_> = paths.0.iter().collect();
            trace!("sorting paths alphabetically by name");
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
                                        let schema = schema.merge_all_of_schema();
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

                        for param in &op.parameters {
                            match param {
                                Parameter::Body(param) => {
                                    let name = format!(
                                        "{}{}Param",
                                        format_type_name(
                                            op.operation_id.as_deref().unwrap_or("InlineResponse")
                                        ),
                                        format_type_name(&param.name)
                                    );
                                    let schema = param.schema.clone().merge_all_of_schema();
                                    self.add_schema_prototype(&name, None, &schema)
                                }
                                _ => {}
                            }
                        }
                    }
                };
            }

            for (name, path) in paths {
                trace!("processing path `{name}`");
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
        } else {
            trace!("no paths to process");
        }
    }

    fn get_ref_schema(&self, ref_: &str) -> Option<&Schema> {
        debug!("getting schema for reference `{ref_}`");
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
        debug!("mapping reference `{ref_}`, required: {is_required}, parent: {parent_name:?}");
        let schema = self.get_ref_schema(ref_)?;
        trace!("got schema {schema:?}");
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
        trace!(
            "mapping schema type, type: {ty}, ref: {ref_:?}, required: {is_required}, parent: {parent_name:?}"
        );
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

    fn handle_schema(
        &mut self,
        name: &str,
        parent_name: Option<&str>,
        schema: &Schema,
        writer: &mut impl std::io::Write,
    ) -> std::io::Result<()> {
        debug!("handling schema {name}, parent: {parent_name:?}");
        trace!("{schema:?}");
        let name = if name.is_empty() {
            schema.name().unwrap_or(
                parent_name
                    .map(|parent_name| format!("{}InlineItem", parent_name))
                    .unwrap_or(name.to_string()),
            )
        } else {
            name.to_string()
        };
        let type_name = format_type_name(&name);
        trace!("mapped name: {name}, type name: {type_name}");

        if schema.properties.is_some() {
            self.handle_props_schema(&name, schema, writer)?
        } else if schema.is_array() {
            self.handle_array_schema(&name, schema, writer)?
        } else if schema.is_enum() {
            self.handle_enum_schema(&name, schema, writer)?
        } else if let Some(ref_) = schema.ref_.as_deref() {
            error!("got unhandled reference schema {ref_}");
        } else if let Some(ty) = self.map_schema_type(schema, None, true, Some(&name)) {
            debug!("handling basic type schema {type_name} = {ty}");
            let ty_str = ty.to_string();

            if type_name == ty_str {
                log::warn!("skipping type alias with same name `{type_name} == {ty_str}`");
                return Ok(());
            }

            if let Some(description) = &schema.description {
                self.print_doc_comment(description, None, writer)?;
            }
            writeln!(writer, "pub type {type_name} = {};\n", ty.to_string())?;
        } else {
            error!("unhandled schema {schema:?}");
        }

        Ok(())
    }

    fn handle_props_schema(
        &mut self,
        name: &str,
        schema: &Schema,
        writer: &mut impl std::io::Write,
    ) -> std::io::Result<()> {
        debug!("handling property schema `{name}`");
        let props = schema.properties.as_ref().unwrap();
        let type_name = format_type_name(&name);
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
            debug!("handling property `{prop}`, required: {is_required}");

            match item {
                Item::Reference(ref_) => {
                    trace!("`{prop}` is a reference to `ref_`");
                    let ty = if let Some(ty) = self.map_reference(ref_, is_required, Some(prop)) {
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
                    trace!("`{prop}` is an object {item:?}");
                    let formatted_var = format_var_name(prop);

                    let prop_ty_name = format!("{type_name}{prop}");

                    let ty = if let Some(ty) =
                        self.map_item_type(it, is_required, Some(&prop_ty_name))
                    {
                        ty
                    } else {
                        RustType::Option(Box::new(RustType::Value))
                    };
                    debug!("mapped type for `{name}` `{prop}` - {ty}");

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
        writeln!(writer, "}}\n")
    }

    fn handle_array_schema(
        &mut self,
        name: &str,
        schema: &Schema,
        writer: &mut impl std::io::Write,
    ) -> std::io::Result<()> {
        debug!("handling array schema `{name}`");
        if let Some(item) = &schema.items {
            let ty = self.map_item_type(&item, true, Some(&name));
            if ty.is_none() {
                return Ok(());
            }
            let ty = ty.unwrap();
            let ty = RustType::Vec(Box::new(ty));
            debug!("mapped type for `{name}` - {ty}");
            let type_name = format_type_name(name);
            let ty_str = ty.to_string();

            if type_name == ty_str {
                log::warn!("skipping type alias with same name `{type_name} == {ty_str}`");
                return Ok(());
            }

            self.print_description(&schema, writer)?;
            writeln!(writer, "pub type {type_name} = {ty_str};\n")?;
        }
        Ok(())
    }

    fn handle_enum_schema(
        &mut self,
        name: &str,
        schema: &Schema,
        writer: &mut impl std::io::Write,
    ) -> std::io::Result<()> {
        debug!("handling enum schema `{name}`");

        let type_name = format_type_name(&name);
        // type declaration

        self.print_derives(&schema, writer)?;
        self.print_description(&schema, writer)?;
        writeln!(writer, "pub enum {type_name} {{")?;
        for enum_value in &schema.enum_ {
            if let Some(val) = enum_value.as_str() {
                writeln!(writer, "    #[serde(rename = \"{val}\")]")?;
                writeln!(writer, "{},", format_enum_value_name(val))?;
            }
        }
        writeln!(writer, "}}\n")?;

        // implement AsRef<str>
        writeln!(writer, "impl AsRef<str> for {type_name} {{")?;
        writeln!(writer, "    fn as_ref(&self) -> &str {{")?;
        writeln!(writer, "        match self {{")?;
        for enum_value in &schema.enum_ {
            if let Some(val) = enum_value.as_str() {
                writeln!(
                    writer,
                    "            {type_name}::{} => \"{val}\",",
                    format_enum_value_name(val)
                )?;
            }
        }
        writeln!(writer, "        }}\n    }}\n}}\n")?;

        // implement Display
        writeln!(
            writer,
            r#"impl std::fmt::Display for {type_name} {{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {{
        write!(f, "{{}}", self.as_ref())
    }}
}}
"#
        )
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
}

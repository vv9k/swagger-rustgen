mod prototyper;

use crate::v2::{items::Item, schema::Schema, Swagger};
use crate::{
    name::{format_enum_value_name, format_type_name, format_var_name},
    types::RustType,
};
use prototyper::{ModelPrototype, Prototyper};

use log::{debug, error, trace};
use std::cmp::Ordering;

pub struct CodeGenerator {
    swagger: Swagger,
    models_to_generate: Vec<ModelPrototype>,
    generated_models: Vec<String>,
}

impl CodeGenerator {
    pub fn new(swagger: Swagger) -> Self {
        Self {
            swagger,
            models_to_generate: vec![],
            generated_models: vec![],
        }
    }

    pub fn generate_models(&mut self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        let swagger = self.swagger.clone();
        let p = Prototyper::default();
        self.models_to_generate = p.generate_prototypes(swagger);
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
                    if let Some(schema) = self.swagger.get_ref_schema(&ref_) {
                        if !schema.is_object() {
                            continue;
                        }
                        if let Some(ty) =
                            self.swagger
                                .map_reference_type(&ref_, true, Some(&model.name))
                        {
                            let type_name = format_type_name(&model.name);
                            let ty_str = ty.to_string();

                            if type_name == ty_str {
                                log::warn!(
                                    "skipping type alias with same name `{type_name} == {ty_str}`"
                                );
                                continue;
                            }

                            if self.generated_models.contains(&type_name) {
                                log::warn!(
                                    "skipping type alias `{type_name}`, a type with the same name already exists"
                                );
                                continue;
                            }
                            self.print_description(&schema, writer)?;
                            writeln!(writer, "pub type {type_name} = {ty_str};\n")?;
                            self.generated_models.push(type_name);
                        }
                    }
                }
                Item::Object(schema) => {
                    let schema = self.swagger.merge_all_of_schema(*schema);
                    self.generate_schema(
                        &model.name,
                        model.parent_name.as_deref(),
                        &schema,
                        writer,
                    )?;
                }
            }
        }
        Ok(())
    }

    fn generate_schema(
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
            self.generate_props_schema(&name, schema, writer)?
        } else if schema.is_array() {
            self.generate_array_schema(&name, schema, writer)?
        } else if schema.is_enum() {
            self.generate_enum_schema(&name, schema, writer)?
        } else if let Some(ref_) = schema.ref_.as_deref() {
            error!("got unhandled reference schema {ref_}");
        } else if let Some(ty) = self
            .swagger
            .map_schema_type(schema, None, true, Some(&name))
        {
            debug!("handling basic type schema {type_name} = {ty}");
            let ty_str = ty.to_string();

            if type_name == ty_str {
                log::warn!("skipping type alias with same name `{type_name} == {ty_str}`");
                return Ok(());
            }
            if self.generated_models.contains(&type_name) {
                log::warn!(
                    "skipping type alias `{type_name}`, a type with the same name already exists"
                );
                return Ok(());
            }

            if let Some(description) = &schema.description {
                self.print_doc_comment(description, None, writer)?;
            }
            writeln!(writer, "pub type {type_name} = {};\n", ty.to_string())?;
            self.generated_models.push(type_name);
        } else {
            error!("unhandled schema {schema:?}");
        }

        Ok(())
    }

    fn generate_props_schema(
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
            let is_required = schema.required.contains(prop);
            debug!("handling property `{prop}`, required: {is_required}");

            match item {
                Item::Reference(ref_) => {
                    trace!("`{prop}` is a reference to `ref_`");
                    let ty = if let Some(ty) =
                        self.swagger
                            .map_reference_type(ref_, is_required, Some(prop))
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
                    trace!("`{prop}` is an object {item:?}");
                    let formatted_var = format_var_name(prop);

                    let prop_ty_name = format!("{type_name}{prop}");

                    let ty = if let Some(ty) =
                        self.swagger
                            .map_item_type(it, is_required, Some(&prop_ty_name))
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
        self.generated_models.push(type_name);
        writeln!(writer, "}}\n")
    }

    fn generate_array_schema(
        &mut self,
        name: &str,
        schema: &Schema,
        writer: &mut impl std::io::Write,
    ) -> std::io::Result<()> {
        debug!("handling array schema `{name}`");
        if let Some(item) = &schema.items {
            let ty = self.swagger.map_item_type(&item, true, Some(&name));
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
            if self.generated_models.contains(&type_name) {
                log::warn!(
                    "skipping type alias `{type_name}`, a type with the same name already exists"
                );
                return Ok(());
            }

            self.print_description(&schema, writer)?;
            writeln!(writer, "pub type {type_name} = {ty_str};\n")?;
            self.generated_models.push(type_name);
        }
        Ok(())
    }

    fn generate_enum_schema(
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
        )?;
        self.generated_models.push(type_name);
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

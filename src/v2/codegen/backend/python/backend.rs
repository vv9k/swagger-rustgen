use crate::v2::codegen::{
    backend::{
        python::{self, format_type_name, format_var_name},
        CodegenBackend,
    },
    ModelPrototype,
};
use crate::v2::{Item, Schema, Swagger};

use log::{debug, error, trace};

#[derive(Default)]
pub struct Codegen {
    generated_models: Vec<String>,
}

impl CodegenBackend<python::Type> for Codegen {
    fn generate_model(
        &mut self,
        model: ModelPrototype,
        swagger: &Swagger<python::Type>,
        writer: &mut Box<dyn std::io::Write>,
    ) -> std::io::Result<()> {
        trace!("generating {} `{}`", model.schema.type_(), &model.name);
        match &model.schema {
            Item::Reference(ref_) => {
                self.generate_reference_model(ref_, &model, swagger, writer)?
            }
            Item::Object(schema) => self.generate_object_model(schema, &model, swagger, writer)?,
        }
        Ok(())
    }

    fn generate_helpers(
        &mut self,
        _swagger: &Swagger<python::Type>,
        writer: &mut Box<dyn std::io::Write>,
    ) -> std::io::Result<()> {
        write!(
            writer,
            r#"
import typing
import json
from typing import List, Dict, TypeAlias, Optional
from dataclasses import dataclass
from json import JSONEncoder, JSONDecoder
"#
        )
    }

    fn generate(
        &mut self,
        swagger: &Swagger<python::Type>,
        writer: &mut Box<dyn std::io::Write>,
    ) -> std::io::Result<()> {
        self.generate_helpers(swagger, writer)?;
        self.generate_forward_declarations(swagger, writer)?;
        self.generate_models(swagger, writer)
    }
}

impl Codegen {
    fn generate_reference_model(
        &mut self,
        ref_: &str,
        model: &ModelPrototype,
        swagger: &Swagger<python::Type>,
        writer: &mut Box<dyn std::io::Write>,
    ) -> std::io::Result<()> {
        if let Some(schema) = swagger.get_ref_schema(ref_) {
            let schema = swagger.merge_all_of_schema(schema.clone());
            if !schema.is_object() {
                return Ok(());
            }
            if let Some(ty) = swagger.map_reference_type(&ref_, true, Some(&model.name)) {
                let type_name = format_type_name(&model.name);
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
                writeln!(writer, "{type_name} = {ty_str}\n")?;
                self.generated_models.push(type_name);
            }
        }
        Ok(())
    }

    fn generate_object_model(
        &mut self,
        schema: &Schema,
        model: &ModelPrototype,
        swagger: &Swagger<python::Type>,
        writer: &mut Box<dyn std::io::Write>,
    ) -> std::io::Result<()> {
        let schema = swagger.merge_all_of_schema(schema.clone());
        self.generate_schema(
            &model.name,
            model.parent_name.as_deref(),
            &schema,
            swagger,
            writer,
        )
    }

    fn generate_schema(
        &mut self,
        name: &str,
        parent_name: Option<&str>,
        schema: &Schema,
        swagger: &Swagger<python::Type>,
        writer: &mut Box<dyn std::io::Write>,
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

        writeln!(writer)?;
        if schema.properties.is_some() {
            self.generate_props_schema(&name, schema, swagger, writer)?
        } else if schema.is_array() {
            self.generate_array_schema(&name, schema, swagger, writer)?
        } else if schema.is_string_enum() {
            self.generate_enum_schema(&name, schema, swagger, writer)?
        } else if let Some(ref_) = schema.ref_.as_deref() {
            error!("got unhandled reference schema {ref_}");
        } else if let Some(ty) = swagger.map_schema_type(schema, None, true, Some(&name)) {
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
            writeln!(writer, "{type_name} = {}\n", ty.to_string())?;
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
        swagger: &Swagger<python::Type>,
        writer: &mut Box<dyn std::io::Write>,
    ) -> std::io::Result<()> {
        debug!("handling property schema `{name}`");
        let props = schema.properties.as_ref().unwrap();
        let type_name = format_type_name(&name);

        struct Prop<'a> {
            comment: Option<&'a String>,
            name: String,
            ty: python::Type,
        }

        let mut props: Vec<_> = props.0.iter().collect();
        props.sort_unstable_by_key(|(k, _)| *k);

        let mut parsed_props = vec![];
        for (prop, item) in &props {
            let is_required = schema.required.contains(prop);
            debug!("handling property `{prop}`");

            match item {
                Item::Reference(ref_) => {
                    trace!("`{prop}` is a reference to `ref_`");
                    let ty = if let Some(ty) =
                        swagger.map_reference_type(ref_, is_required, Some(prop))
                    {
                        ty
                    } else {
                        python::Type::Value
                    };
                    let name = format_var_name(prop);
                    parsed_props.push(Prop {
                        comment: None,
                        name,
                        ty,
                    });
                }
                it @ Item::Object(item) => {
                    trace!("`{prop}` is an object {item:?}");
                    let name = format_var_name(prop);

                    let prop_ty_name = format!("{type_name}_{prop}");

                    let ty = if let Some(ty) =
                        swagger.map_item_type(it, is_required, Some(&prop_ty_name))
                    {
                        ty
                    } else {
                        python::Type::Value
                    };
                    debug!("mapped type for `{name}` `{prop}` - {ty}");

                    parsed_props.push(Prop {
                        comment: item.description.as_ref(),
                        name,
                        ty,
                    });
                }
            }
        }

        let mut required = vec![];
        let mut optional = vec![];
        let mut has_comments = schema.description.is_some();
        parsed_props.into_iter().for_each(|prop| {
            if prop.comment.is_some() {
                has_comments = true;
            }
            if let python::Type::Optional(_) = prop.ty {
                optional.push(prop)
            } else {
                required.push(prop)
            }
        });

        self.print_json_encoders(&type_name, writer)?;

        writeln!(writer, "@dataclass")?;
        writeln!(writer, "class {type_name}:")?;

        if has_comments {
            writeln!(writer, "    \"\"\"")?;
        }
        if let Some(description) = &schema.description {
            for line in description.lines() {
                writeln!(writer, "{line}")?;
            }
        }

        if !required.is_empty() && has_comments {
            writeln!(writer)?;
            writeln!(writer, "Required properties:")?;
        }
        for prop in &required {
            if let Some(comment) = prop.comment {
                writeln!(
                    writer,
                    "    * {}: {}",
                    prop.name,
                    comment.replace("\"", "'")
                )?;
            }
        }
        if !optional.is_empty() && has_comments {
            writeln!(writer)?;
            writeln!(writer, "Optional properties:")?;
        }
        for prop in &optional {
            if let Some(comment) = prop.comment {
                writeln!(
                    writer,
                    "    * {}: {}",
                    prop.name,
                    comment.replace("\"", "'")
                )?;
            }
        }
        if has_comments {
            writeln!(writer, "    \"\"\"")?;
        }

        for prop in &required {
            writeln!(writer, "    {}: {}", prop.name, prop.ty)?;
        }
        for prop in &optional {
            writeln!(writer, "    {}: {} = None", prop.name, prop.ty)?;
        }

        writeln!(writer)?;
        writeln!(writer, "    @staticmethod")?;

        writeln!(writer, "    def from_json(data) -> {type_name}:")?;
        writeln!(
            writer,
            "        return json.loads(data, cls={type_name}JsonDecoder)"
        )?;
        writeln!(writer)?;
        writeln!(writer, "    def to_json(self) -> str:")?;
        writeln!(
            writer,
            "        return json.dumps(self, cls={type_name}JsonEncoder)"
        )?;

        self.generated_models.push(type_name);
        Ok(())
    }

    fn generate_array_schema(
        &mut self,
        name: &str,
        schema: &Schema,
        swagger: &Swagger<python::Type>,
        writer: &mut Box<dyn std::io::Write>,
    ) -> std::io::Result<()> {
        debug!("handling array schema `{name}`");
        if let Some(item) = &schema.items {
            let ty = swagger.map_item_type(&item, true, Some(&name));
            if ty.is_none() {
                return Ok(());
            }
            let ty = ty.unwrap();
            let ty = python::Type::List(Box::new(ty));
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

            self.print_json_encoders(&type_name, writer)?;
            self.print_description(&schema, writer)?;
            writeln!(writer, "{type_name}: TypeAlias = \"{ty_str}\"\n")?;
            self.generated_models.push(type_name);
        }
        Ok(())
    }

    fn generate_enum_schema(
        &mut self,
        name: &str,
        _schema: &Schema,
        _swagger: &Swagger<python::Type>,
        _writer: &mut Box<dyn std::io::Write>,
    ) -> std::io::Result<()> {
        debug!("handling enum schema `{name}`");

        let type_name = format_type_name(&name);
        self.generated_models.push(type_name);
        Ok(())
    }

    fn print_description(
        &self,
        schema: &Schema,
        writer: &mut Box<dyn std::io::Write>,
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
        writer: &mut Box<dyn std::io::Write>,
    ) -> std::io::Result<()> {
        let indentation = indentation
            .map(|i| " ".repeat(i.into()))
            .unwrap_or_default();
        for line in comment.as_ref().lines() {
            writeln!(writer, "{indentation}# {line}")?;
        }
        Ok(())
    }

    fn print_json_encoders(
        &self,
        ty: &str,
        writer: &mut Box<dyn std::io::Write>,
    ) -> std::io::Result<()> {
        write!(
            writer,
            "{}",
            format!(
                "
class {ty}JsonEncoder(JSONEncoder):
    def default(self, o):
        return {{k: v for k, v in o.__dict__.items() if v is not None}}
class {ty}JsonDecoder(JSONDecoder):
    def __init__(self):
        JSONDecoder.__init__(self, object_hook={ty}JsonDecoder.from_dict)

    @staticmethod
    def from_dict(d):
        return {ty}(**d)
"
            )
        )
    }

    pub fn generate_forward_declarations(
        &mut self,
        swagger: &Swagger<python::Type>,
        writer: &mut Box<dyn std::io::Write>,
    ) -> std::io::Result<()> {
        let prototypes = self.prototypes(swagger);
        writeln!(writer)?;

        for prototype in prototypes {
            let name = if prototype.name.is_empty() {
                if let Some(parent) = prototype.parent_name {
                    format!("{parent}InlineItem")
                } else {
                    continue;
                }
            } else {
                prototype.name
            };
            let type_name = format_type_name(&name);
            writeln!(
                writer,
                "{type_name} = typing.NewType(\"{type_name}\", None)"
            )?;
        }
        Ok(())
    }
}

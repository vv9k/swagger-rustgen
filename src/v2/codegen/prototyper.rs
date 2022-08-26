use crate::name::format_type_name;
use crate::v2::{
    items::Item, parameter::Parameter, path::Path, responses::Response, schema::Schema, Swagger,
};

use log::{debug, error, trace};

#[derive(Debug)]
pub struct ModelPrototype {
    pub name: String,
    pub parent_name: Option<String>,
    pub schema: Item,
}

#[derive(Debug)]
pub struct Prototyper {
    prototypes: Vec<ModelPrototype>,
}

impl Default for Prototyper {
    fn default() -> Self {
        Self { prototypes: vec![] }
    }
}

impl Prototyper {
    pub fn generate_prototypes(mut self, swagger: Swagger) -> Vec<ModelPrototype> {
        self.add_definition_models(&swagger);
        self.add_responses_models(&swagger);
        self.add_paths_models(&swagger);
        self.prototypes
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
        self.prototypes.push(prototype);
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
        self.prototypes.push(prototype);
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
                let schema = swagger.merge_all_of_schema(schema.clone());
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
                            let schema = swagger.merge_all_of_schema(schema.clone());
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
                                        let schema = swagger.merge_all_of_schema(schema.clone());
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
                                    let schema = swagger.merge_all_of_schema(param.schema.clone());
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
}

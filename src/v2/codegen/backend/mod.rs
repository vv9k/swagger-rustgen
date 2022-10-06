pub mod python;
pub mod rust;

use crate::v2::{
    codegen::{ModelPrototype, Prototyper},
    Swagger, Type,
};

use std::cmp::Ordering;

pub trait CodegenBackend<T: Type> {
    fn generate_model(
        &mut self,
        model: ModelPrototype,
        swagger: &Swagger<T>,
        writer: &mut Box<dyn std::io::Write>,
    ) -> std::io::Result<()>;

    fn generate_helpers(
        &mut self,
        swagger: &Swagger<T>,
        writer: &mut Box<dyn std::io::Write>,
    ) -> std::io::Result<()>;

    fn prototypes(&self, swagger: &Swagger<T>) -> Vec<ModelPrototype> {
        let p = Prototyper::default();
        let mut prototypes = p.generate_prototypes(swagger);

        // Generate object schemas first so that all references are valid
        // and fallback to alphabetical sorting
        prototypes.sort_by(
            |a, b| match (a.schema.is_reference(), b.schema.is_reference()) {
                (true, true) | (false, false) => a.name.cmp(&b.name),
                (true, false) => Ordering::Greater,
                (false, true) => Ordering::Less,
            },
        );
        prototypes
    }

    fn generate_models(
        &mut self,
        swagger: &Swagger<T>,
        writer: &mut Box<dyn std::io::Write>,
    ) -> std::io::Result<()> {
        let prototypes = self.prototypes(swagger);

        for prototype in prototypes {
            self.generate_model(prototype, swagger, writer)?;
        }

        Ok(())
    }

    fn generate(
        &mut self,
        swagger: &Swagger<T>,
        writer: &mut Box<dyn std::io::Write>,
    ) -> std::io::Result<()> {
        self.generate_helpers(swagger, writer)?;
        self.generate_models(swagger, writer)
    }
}

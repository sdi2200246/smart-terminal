use serde_json::{Value};
use schemars::{JsonSchema};
use schemars::generate::SchemaSettings;

pub trait FlatSchema : JsonSchema{
    fn schema() -> Value {
        let settings = SchemaSettings::draft07().with(|s| {
            s.inline_subschemas = true;
        });
        let r#gen = settings.into_generator();
        let schema = r#gen.into_root_schema_for::<Self>();
        serde_json::to_value(schema).unwrap()
    }
}
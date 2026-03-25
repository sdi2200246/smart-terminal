use serde_json::{Value};
use schemars::{JsonSchema};
use schemars::generate::SchemaSettings;

pub trait FlatSchema : JsonSchema{
    fn schema() -> Value {
        // 1. Create settings that force inlining
        let settings = SchemaSettings::draft07().with(|s| {
            s.inline_subschemas = true;
        });
        
        // 2. Create a generator with those settings
        let r#gen = settings.into_generator();
        
        // 3. Generate the schema for the current type
        let schema = r#gen.into_root_schema_for::<Self>();
        
        // 4. Convert to serde_json::Value
        serde_json::to_value(schema).unwrap()
    }

}
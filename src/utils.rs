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

#[cfg(test)]
mod tests {
    use super::*;
    use schemars::JsonSchema;
    use serde::{Serialize, Deserialize};

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[schemars(deny_unknown_fields)]
    struct Inner {
        value: i32,
    }

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[schemars(deny_unknown_fields)]
    struct Outer {
        name: String,
        inner: Inner,
        list: Vec<Inner>,
    }

    impl FlatSchema for Outer {}

    #[test]
    fn strict_schema_adds_additional_properties_everywhere() {
        let schema = Outer::schema();

        // Debug print if needed
        println!("{}", serde_json::to_string_pretty(&schema).unwrap());

        // Root should be strict
        assert_eq!(
            schema.get("additionalProperties"),
            Some(&serde_json::Value::Bool(false))
        );

        let properties = schema.get("properties").unwrap();

        // Inner object
        let inner = properties.get("inner").unwrap();
        assert_eq!(
            inner.get("additionalProperties"),
            Some(&serde_json::Value::Bool(false))
        );

        // Array items
        let list = properties.get("list").unwrap();
        let items = list.get("items").unwrap();

        assert_eq!(
            items.get("additionalProperties"),
            Some(&serde_json::Value::Bool(false))
        );
    }
}
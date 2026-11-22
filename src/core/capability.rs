use crate::tools::ToolError;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Copy, Default)]
#[serde(rename_all = "snake_case")]
pub enum ToolSafety {
    #[default]
    Safe,
    Mutating,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct ToolMetaData {
    pub name: String,
    pub description: String,
    pub parameters: Value,
    #[serde(default)]
    pub safety: ToolSafety,
}

pub trait Capability: Send + Sync {
    fn name(&self) -> &'static str;
    fn metadata(&self) -> ToolMetaData;
    fn prompt_guidance(&self) -> Option<&'static str> {
        None
    }
    fn execute(&self, args: Value) -> Result<String, ToolError>;
}

pub fn parse_tool_arguments<T>(args: Value) -> Result<T, ToolError>
where
    T: serde::de::DeserializeOwned,
{
    let args = if args.is_null() {
        Value::Object(serde_json::Map::new())
    } else {
        args
    };

    serde_json::from_value(args).map_err(|error| ToolError::ArgumentsParsing {
        source: anyhow::anyhow!("invalid arguments: {error}"),
    })
}

#[derive(JsonSchema, Deserialize, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct NoArgs {}

#[macro_export]
macro_rules! define_tool {
    (
        pub struct $name:ident;
        $(safety = $safety:expr;)?
        $(guidance = $guidance:expr;)?
        #[doc = $first_doc:expr] $(#[doc = $other_docs:expr])*
        fn $fn_name:ident($arg:ident : $args_ty:ty) -> Result<String, ToolError> $body:block
    ) => {
        pub struct $name;

        impl $name {
            pub fn run(&self, $arg: $args_ty) -> Result<String, $crate::tools::ToolError> {
                $body
            }
        }

        impl $crate::core::Capability for $name {
            fn name(&self) -> &'static str {
                stringify!($fn_name)
            }

            fn metadata(&self) -> $crate::core::ToolMetaData {
                $crate::core::ToolMetaData {
                    name: self.name().into(),
                    description: concat!($first_doc $(, "\n", $other_docs)*).into(),
                    parameters: <$args_ty as $crate::utils::FlatSchema>::schema(),
                    safety: $crate::define_tool!(@safety $($safety)?),
                }
            }

            fn prompt_guidance(&self) -> Option<&'static str> {
                None $(.or(Some($guidance)))?
            }

            fn execute(&self, args: serde_json::Value) -> Result<String, $crate::tools::ToolError> {
                let typed = $crate::core::parse_tool_arguments::<$args_ty>(args)?;
                self.run(typed)
            }
        }
    };

    (
        pub struct $name:ident;
        $(safety = $safety:expr;)?
        $(guidance = $guidance:expr;)?
        #[doc = $first_doc:expr] $(#[doc = $other_docs:expr])*
        fn $fn_name:ident() -> Result<String, ToolError> $body:block
    ) => {
        pub struct $name;

        impl $name {
            pub fn run(&self) -> Result<String, $crate::tools::ToolError> {
                $body
            }
        }

        impl $crate::core::Capability for $name {
            fn name(&self) -> &'static str {
                stringify!($fn_name)
            }

            fn metadata(&self) -> $crate::core::ToolMetaData {
                $crate::core::ToolMetaData {
                    name: self.name().into(),
                    description: concat!($first_doc $(, "\n", $other_docs)*).into(),
                    parameters: <$crate::core::NoArgs as $crate::utils::FlatSchema>::schema(),
                    safety: $crate::define_tool!(@safety $($safety)?),
                }
            }

            fn prompt_guidance(&self) -> Option<&'static str> {
                None $(.or(Some($guidance)))?
            }

            fn execute(&self, _args: serde_json::Value) -> Result<String, $crate::tools::ToolError> {
                self.run()
            }
        }
    };

    (@safety $safety:expr) => { $safety };
    (@safety) => { $crate::core::ToolSafety::Safe };
}

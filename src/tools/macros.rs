use schemars::JsonSchema;
use serde::Deserialize;

#[derive(JsonSchema, Deserialize, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct NoArgs {}

#[macro_export]
macro_rules! define_tool {
    // ── 1. With typed arguments ──────────────────────────────────────────────
    (
        pub struct $name:ident;
        $(safety = $safety:expr;)?
        $(guidance = $guidance:expr;)?
        #[doc = $first_doc:expr] $(#[doc = $other_docs:expr])*
        fn $fn_name:ident($arg:ident : $args_ty:ty) -> Result<String, ToolError> $body:block
    ) => {
        pub struct $name;

        impl $name {
            pub fn run(&self, $arg: $args_ty) -> Result<String, ToolError> {
                $body
            }
        }

        impl Capability for $name {
            fn name(&self) -> &'static str {
                stringify!($fn_name)
            }

            fn metadata(&self) -> ToolMetaData {
                ToolMetaData {
                    name: self.name().into(),
                    description: concat!($first_doc $(, "\n", $other_docs)*).into(),
                    parameters: <$args_ty as $crate::utils::FlatSchema>::schema(),
                    safety: $crate::define_tool!(@safety $($safety)?),
                }
            }

            fn prompt_guidance(&self) -> Option<&'static str> {
                None $(.or(Some($guidance)))?
            }

            fn execute(&self, args: Value) -> Result<String, ToolError> {
                let typed: $args_ty = if args.is_null() {
                    serde_json::from_value(Value::Object(serde_json::Map::new()))
                } else {
                    serde_json::from_value(args)
                }
                .map_err(|e| ToolError::ArgumentsParsing {
                    source: anyhow::anyhow!("[ERROR] {}", e),
                })?;

                self.run(typed)
            }
        }
    };

    // ── 2. Zero arguments ────────────────────────────────────────────────────
    (
        pub struct $name:ident;
        $(safety = $safety:expr;)?
        $(guidance = $guidance:expr;)?
        #[doc = $first_doc:expr] $(#[doc = $other_docs:expr])*
        fn $fn_name:ident() -> Result<String, ToolError> $body:block
    ) => {
        pub struct $name;

        impl $name {
            pub fn run(&self) -> Result<String, ToolError> {
                $body
            }
        }

        impl Capability for $name {
            fn name(&self) -> &'static str {
                stringify!($fn_name)
            }

            fn metadata(&self) -> ToolMetaData {
                ToolMetaData {
                    name: self.name().into(),
                    description: concat!($first_doc $(, "\n", $other_docs)*).into(),
                    parameters: <$crate::tools::NoArgs as $crate::utils::FlatSchema>::schema(),
                    safety: $crate::define_tool!(@safety $($safety)?),
                }
            }

            fn prompt_guidance(&self) -> Option<&'static str> {
                None $(.or(Some($guidance)))?
            }

            fn execute(&self, _args: Value) -> Result<String, ToolError> {
                self.run()
            }
        }
    };

    (@safety $safety:expr) => { $safety };
    (@safety) => { $crate::core::ToolSafety::Safe };
}

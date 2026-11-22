#[macro_export]
macro_rules! define_signal {
    (
        pub struct $name:ident;
        $(safety = $safety:expr;)?
        $(guidance = $guidance:expr;)?
        $(must_run_alone = $alone:expr;)?
        #[doc = $first_doc:expr] $(#[doc = $other_docs:expr])*
        fn $fn_name:ident($arg:ident : $args_ty:ty, $ctx:ident : &mut AgentSignalContext<'_>) -> Result<AgentSignalOutcome, ToolError> $body:block
    ) => {
        pub struct $name;

        impl $name {
            pub fn handle_signal(&self, $arg: $args_ty, $ctx: &mut $crate::agent::signals::AgentSignalContext<'_>) -> Result<$crate::agent::signals::AgentSignalOutcome, $crate::tools::ToolError> {
                $body
            }
        }

        impl $crate::agent::signals::AgentSignal for $name {
            fn name(&self) -> &'static str {
                stringify!($fn_name)
            }

            fn metadata(&self) -> $crate::core::ToolMetaData {
                $crate::core::ToolMetaData {
                    name: self.name().into(),
                    description: concat!($first_doc $(, "\n", $other_docs)*).into(),
                    parameters: <$args_ty as $crate::utils::FlatSchema>::schema(),
                    safety: $crate::define_signal!(@safety $($safety)?),
                }
            }

            fn prompt_guidance(&self) -> Option<&'static str> {
                None $(.or(Some($guidance)))?
            }

            fn must_run_alone(&self) -> bool {
                false $(|| $alone)?
            }

            fn handle(&self, args: serde_json::Value, ctx: &mut $crate::agent::signals::AgentSignalContext<'_>) -> Result<$crate::agent::signals::AgentSignalOutcome, $crate::tools::ToolError> {
                let action_name = stringify!($fn_name);
                let subject = action_name.strip_prefix("update_").unwrap_or(action_name);

                let typed_args: $args_ty = if args.is_null() {
                    serde_json::from_value(serde_json::Value::Object(serde_json::Map::new()))
                } else {
                    serde_json::from_value(args)
                }
                .map_err(|e| $crate::tools::ToolError::ArgumentsParsing {
                    source: anyhow::anyhow!("invalid {subject} — {e}"),
                })?;

                self.handle_signal(typed_args, ctx)
            }
        }
    };

    (@safety $safety:expr) => {
        $safety
    };
    (@safety) => {
        $crate::core::ToolSafety::Safe
    };
}

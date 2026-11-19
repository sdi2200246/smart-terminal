use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::utils::FlatSchema;

#[derive(JsonSchema, Deserialize, Serialize, Debug)]
#[schemars(deny_unknown_fields)]
pub enum Reversibility {
    Full,
    Mostly,
    Partial,
    Hard,
    Irreversible,
}

#[derive(JsonSchema, Deserialize, Serialize, Debug)]
#[schemars(deny_unknown_fields)]
pub struct NextCommand {
    /// The exact shell command to run. May span multiple lines; no Markdown fences or `$ ` prefix.
    pub cmd: String,
    /// One short, single-line sentence describing the command and any assumption made. Under 15 words.
    pub man: String,
    /// How reversible the command is, given the current environment.
    pub scale: Reversibility,
}
impl FlatSchema for NextCommand {}

impl fmt::Display for NextCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let json = serde_json::to_string_pretty(self).map_err(|_| fmt::Error)?;
        f.write_str(&json)
    }
}

#[derive(JsonSchema, Deserialize, Serialize, Debug, Clone)]
#[schemars(deny_unknown_fields)]
pub struct PlanStep {
    /// Concrete investigation step: what to look at, what command to run, or what file to read. One file, one command, or one directory per step.
    pub action: String,
    /// Why this step advances the answer to the user's question.
    pub rationale: String,
}

#[derive(JsonSchema, Deserialize, Serialize, Debug, Clone)]
#[schemars(deny_unknown_fields)]
pub struct Plan {
    /// One-line restatement of the user's question.
    pub goal: String,
    /// ordered, atomic investigation steps grounded in directory paths verified to exist.
    pub steps: Vec<PlanStep>,
}
impl FlatSchema for Plan {}

impl fmt::Display for Plan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let json = serde_json::to_string_pretty(self).map_err(|_| fmt::Error)?;
        f.write_str(&json)
    }
}

#[derive(JsonSchema, Deserialize, Serialize, Debug)]
#[schemars(deny_unknown_fields)]
pub struct Report {
    ///A direct text with out special characters report answering the user's question. Not a description of what was done.
    pub report: String,
    /// The most likely next command the user wants to run, grounded in the investigation.
    pub recommended_command: NextCommand,
}
impl FlatSchema for Report {}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let json = serde_json::to_string_pretty(self).map_err(|_| fmt::Error)?;
        f.write_str(&json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_requires_a_recommended_command() {
        let report = serde_json::json!({
            "report": "The working tree has no changes.",
            "recommended_command": {
                "cmd": "git status --short",
                "man": "Verify that the working tree remains clean.",
                "scale": "Full"
            }
        });
        assert!(serde_json::from_value::<Report>(report).is_ok());

        let report_without_command = serde_json::json!({
            "report": "The working tree has no changes."
        });
        assert!(serde_json::from_value::<Report>(report_without_command).is_err());
    }
}

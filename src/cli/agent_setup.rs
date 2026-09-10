use serde_json::Value;

use crate::agent::workflows::investigator::InvestigatorToolFactory;
use crate::agent::workflows::next_cmd::NextCmdToolFactory;
use crate::core::capability::Capability;

use crate::tools::bash::Bash;
use crate::tools::docker::Docker;
use crate::tools::git_diff::GitDiffStaged;
use crate::tools::json::Json;
use crate::tools::last_error::ReadLastError;
use crate::tools::read_dir::ReadDir;
use crate::tools::read_file::ReadFile;

pub struct CliToolProvider;

impl InvestigatorToolFactory for CliToolProvider {
    fn planner_tools(&self) -> Vec<Box<dyn Capability>> {
        return vec![Box::new(ReadDir)];
    }

    fn executor_tools(&self) -> Vec<Box<dyn Capability>> {
        return vec![Box::new(ReadDir), Box::new(Bash), Box::new(ReadFile)];
    }
}

impl NextCmdToolFactory for CliToolProvider {
    fn cmd_predictor_tools(&self, schema: Value) -> Vec<Box<dyn Capability>> {
        return vec![
            Box::new(GitDiffStaged),
            Box::new(Docker),
            Box::new(Json { properties: schema }),
            Box::new(ReadLastError),
        ];
    }
}

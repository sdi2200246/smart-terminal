mod integration {
    mod google;
    mod grog;
    mod investigator;
    mod next_cmd;
    use serde_json::Value;
    use smart_terminal::agent::workflows::investigator::InvestigatorToolFactory;
    use smart_terminal::agent::workflows::next_cmd::NextCmdToolFactory;
    use smart_terminal::core::capability::Capability;
    use smart_terminal::tools::bash::Bash;
    use smart_terminal::tools::docker::Docker;
    use smart_terminal::tools::git_diff::GitDiffStaged;
    use smart_terminal::tools::json::Json;
    use smart_terminal::tools::last_error::ReadLastError;
    use smart_terminal::tools::read_dir::ReadDir;
    use smart_terminal::tools::read_file::ReadFile;

    pub struct TestToolProvider;

    impl InvestigatorToolFactory for TestToolProvider {
        fn planner_tools(&self) -> Vec<Box<dyn Capability>> {
            return vec![Box::new(ReadDir)];
        }

        fn executor_tools(&self) -> Vec<Box<dyn Capability>> {
            return vec![Box::new(ReadDir), Box::new(Bash), Box::new(ReadFile)];
        }
    }

    impl NextCmdToolFactory for TestToolProvider {
        fn cmd_predictor_tools(&self, schema: Value) -> Vec<Box<dyn Capability>> {
            return vec![
                Box::new(GitDiffStaged),
                Box::new(Docker),
                Box::new(Json { properties: schema }),
                Box::new(ReadLastError),
            ];
        }
    }
}

mod integration {
    mod google;
    mod grog;
    mod investigator;
    mod next_cmd;
    use smart_terminal::agent::{InvestigatorToolFactory, NextCmdToolFactory};
    use smart_terminal::core::Capability;
    use smart_terminal::tools::{Bash, Docker, GitDiffStaged, ReadDir, ReadFile, ReadLastError};

    pub struct TestToolProvider;

    impl InvestigatorToolFactory for TestToolProvider {
        fn planner_tools(&self) -> Vec<Box<dyn Capability>> {
            vec![Box::new(ReadDir)]
        }

        fn executor_tools(&self) -> Vec<Box<dyn Capability>> {
            vec![Box::new(ReadDir), Box::new(Bash), Box::new(ReadFile)]
        }
    }

    impl NextCmdToolFactory for TestToolProvider {
        fn cmd_predictor_tools(&self) -> Vec<Box<dyn Capability>> {
            vec![
                Box::new(GitDiffStaged),
                Box::new(Docker),
                Box::new(ReadLastError),
            ]
        }
    }
}

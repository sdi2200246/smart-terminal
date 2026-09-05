use crate::agent::patterns::react::ReactLoop;
use crate::agent::memory::FolderMemory;
use crate::agent::workflows::next_cmd::{NextCmd, Reversibility};
use crate::cli::cli::NextCmdArgs;
use crate::core::memory::Memory;
use crate::providers::groq::client::GroqClient;
use crate::cli::agent_setup::CliToolProvider;
use std::env;
use std::io::{self, Write};

pub async fn run(args: NextCmdArgs) {
    let mut memory =
        FolderMemory::project_local().unwrap_or_else(|_| FolderMemory::new(env::temp_dir()));

    if let Ok(cwd) = env::current_dir() {
        let _ = memory.load(&cwd);
    }

    let provider = GroqClient::pooled();
    let mut runner = ReactLoop::new(provider);

    let prediction = {
        let mut workflow = NextCmd::new(&mut runner, &mut memory , CliToolProvider);
        match workflow.run(args.buffer).await {
            Ok(p) => p,
            Err(e) => {
                println!("< agent failed >");
                println!("{e}");
                println!("{:?}", Reversibility::Irreversible);
                io::stdout().flush().unwrap();
                return;
            }
        }
    };
    println!("{}", prediction.cmd);
    println!("{}", prediction.man);
    println!("{:?}", prediction.scale);

    io::stdout().flush().unwrap();
}

use crate::cli::adapters::{AgentIntent};
use crate::agent::archtectures::react::ReactLoop;
use crate::cli::cli::NextCmdArgs;
use crate::groq::client::GroqClient;
 
pub async fn run(args: NextCmdArgs) {
    let intent = AgentIntent::from(args);
    let provider = GroqClient::pooled();
    let agent_loop = ReactLoop::new(provider);
 
    // let mut agent = AgentClient::new("NEXT_CMD_AGENT", provider, agent_loop);
    // let (session, tools) = Policy::build(&intent);
    // let response = agent.execute(session, tools).await;
 
    // match response {
    //     AgentResponse::Success(value) => {
    //         let suggestion: NextCommand = serde_json::from_value(value).unwrap();
    //         println!("{}", &suggestion.cmd);
    //         println!("{}", &suggestion.man);
    //         println!("{:?}", &suggestion.scale);
    //     }
    //     _ => {}
    // }
}
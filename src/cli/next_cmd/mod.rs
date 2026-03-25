mod policy;
use policy::{Policy , NextCommand};
use crate::agent::request::AgentIntent;
use crate::agent::responce::AgentResponse;
use crate::agent::client::AgentClient;
use crate::cli::cli::NextCmdArgs;
use crate::groq::client::GroqClient;
use crate::agent::loops::react::ReactLoop;

pub async fn run(args:NextCmdArgs){

    let itend = AgentIntent::from(args);
    let provider = GroqClient::default();
    let agent_loop = ReactLoop;

    let mut agent = AgentClient::new("NEXT_CMD_AGENT", provider, agent_loop);

    let policy = Policy::select_policy();
    let req = policy.create_req(itend);
    let response = agent.execute_request(req).await;


    match response {
        AgentResponse::Success(value) => {
            let suggestion: NextCommand = serde_json::from_value(value).unwrap();

            println!("{}" , &suggestion.cmd);
            println!("{}" , &suggestion.man);

        }
        _=>{}
    }
}
 
#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn run_with_align_true() {
        let args = NextCmdArgs {
            buffer: "gcc age".to_string(),
        };
        run(args).await;
    }
}       
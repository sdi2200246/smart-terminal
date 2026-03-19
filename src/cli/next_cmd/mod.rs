mod policy;
use policy::{Policy , Command};
use crate::agent::service::AgentService;
use crate::agent::responce::AgentResponse;
use crate::cli::cli::NextCmdArgs;
use crate::groq::client::GroqClient;
use crate::agent::loops::react::ReactLoop;
use tokio::sync::mpsc;

pub async fn run(args:NextCmdArgs){


    let client = GroqClient::default();
    let agent_type = ReactLoop{};
    let tx = AgentService::spawn("NextCMD_Agent".into() , client , agent_type);
    let (response_tx, mut response_rx) = mpsc::channel(1);

    let policy = Policy::select_policy();
    let req = policy.create_req(args, response_tx);

    tx.send(req).await.unwrap();

    let response = response_rx.recv().await.unwrap();

    match response {
        AgentResponse::Success(value) => {
            let suggestion: Command = serde_json::from_value(value).unwrap();

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
            buffer: "git commit -m".to_string(),
        };
        run(args).await;
    }

}       
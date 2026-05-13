// mod policy;
// use policy::{Policy, Script};
// use serde_json::Value;
 
// use super::cli::ExecArgs;
// use super::adapters::AgentIntent;
// use crate::agent::responce::AgentResponse;
// use crate::agent::client::AgentClient;
// use crate::groq::client::GroqClient;
// use crate::core::session::{Model, ModelName};
 
// fn render_success(stdout: &str) {
//     println!("\x1b[32m✓ Success\x1b[0m");
//     if !stdout.is_empty() {
//         println!("{stdout}");
//     }
// }
 
// fn render_error(stderr: &str) {
//     eprintln!("\x1b[31m✗ Failed\x1b[0m");
//     if !stderr.is_empty() {
//         eprintln!("{stderr}");
//     }
// }
 
// fn evaluation_script(response: &Value) -> Option<String> {
//     let script: Script = serde_json::from_value(response.clone()).ok()?;
 
//     if script.script.is_empty() {
//         return Some("script is empty".into());
//     }
 
//     let syntax_check = std::process::Command::new("bash")
//         .arg("-n")
//         .arg("-c")
//         .arg(&script.script)
//         .output()
//         .ok()?;
 
//     if !syntax_check.status.success() {
//         let err = String::from_utf8_lossy(&syntax_check.stderr).to_string();
//         return Some(format!("syntax error: {err}"));
//     }
 
//     println!("Testing taking place ... ");
 
//     let output = std::process::Command::new("bash")
//         .arg("-c")
//         .arg(&script.script)
//         .output()
//         .ok()?;
 
//     let stderr = String::from_utf8_lossy(&output.stderr).to_string();
 
//     if !stderr.is_empty() {
//         let first_three = stderr.lines().take(3).collect::<Vec<_>>().join("\n");
//         return Some(format!("runtime error:\n{first_three}"));
//     }
 
//     if !output.status.success() {
//         let stdout = String::from_utf8_lossy(&output.stdout).to_string();
//         let first_three = stdout.lines().take(3).collect::<Vec<_>>().join("\n");
//         return Some(format!("script failed:\n{first_three}"));
//     }
 
//     None
// }
 
// pub async fn run(args: ExecArgs) {
//     let intent = AgentIntent::from(args);
//     let provider = GroqClient::default();
//     let agent_loop = ReflexionLoop::new(
//         evaluation_script,
//         Model::deterministic(ModelName::GptOss120B),
//     );
 
//     let mut agent = AgentClient::new("SHELL_AGENT", provider, agent_loop);
//     let (session, tools) = Policy::build(&intent);
//     let response = agent.execute(session, tools).await;
 
//     match response {
//         AgentResponse::Success(value) => {
//             let script: Script = serde_json::from_value(value).unwrap();
//             let status = std::process::Command::new("bash")
//                 .arg("-c")
//                 .arg(script.script)
//                 .status()
//                 .expect("failed to execute script");
 
//             if status.success() {
//                 render_success("");
//             } else {
//                 render_error("script failed");
//             }
//         }
//         AgentResponse::Error(e) => {
//             render_error(&e.to_string());
//             std::process::exit(1);
//         }
//     }
// }
 
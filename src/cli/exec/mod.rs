mod policy;
use policy::{Policy , Script};
use serde_json::Value;

use super::cli::ExecArgs;
use crate::agent::responce::AgentResponse;
use crate::agent::loops::react::ReactLoop;
use crate::agent::loops::reflect::ReflexionLoop;
use crate::agent::client::AgentClient;
use crate::groq::client::GroqClient;
use crate::interfaces::policy::AgentIntent;


fn render_success(stdout: &str) {
    println!("\x1b[32m✓ Success\x1b[0m");
    if !stdout.is_empty() {
        println!("{stdout}");
    }
}

fn render_error(stderr: &str) {
    eprintln!("\x1b[31m✗ Failed\x1b[0m");
    if !stderr.is_empty() {
        eprintln!("{stderr}");
    }
}
fn evaluation_script(response: &Value) -> Option<String> {
    let script: Script = serde_json::from_value(response.clone()).ok()?;

    if script.script.is_empty() {
        return Some("script is empty".into());
    }

    // syntax check
    let syntax_check = std::process::Command::new("bash")
        .arg("-n")
        .arg("-c")
        .arg(&script.script)
        .output()
        .ok()?;

    if !syntax_check.status.success() {
        let err = String::from_utf8_lossy(&syntax_check.stderr).to_string();
        return Some(format!("syntax error: {err}"));
    }

    println!("Testing taking place ... ");

    let output = std::process::Command::new("bash")
        .arg("-c")
        .arg(&script.script)
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !stdout.is_empty() {
        println!("{stdout}");
    }

    if !stderr.is_empty() {
        let first_three = stderr
            .lines()
            .take(3)
            .collect::<Vec<_>>()
            .join("\n");
        return Some(format!("runtime error:\n{first_three}"));
    }

    if !output.status.success() {
        let first_three = stdout
            .lines()
            .take(3)
            .collect::<Vec<_>>()
            .join("\n");
        return Some(format!("script failed:\n{first_three}"));
    }

    None
}

pub async fn run(args:ExecArgs){

    let itend = AgentIntent::from(args);
    let provider = GroqClient::default();
    let agent_loop = ReflexionLoop::new(evaluation_script);

    let mut agent = AgentClient::new("SHELL_AGENT", provider, agent_loop);

    let policy = Policy::select_policy(&itend);
    let req = policy.create_req(itend, agent.response_sender());
    let response = agent.execute_request(req).await;


    match response {
        AgentResponse::Success(value) => {
            let script: Script = serde_json::from_value(value).unwrap();
            let status = std::process::Command::new("bash")
                .arg("-c")
                .arg(script.script)
                .status()
                .expect("failed to execute script");

            if status.success() {
                render_success("");
            } else {
                render_error("script failed");
            }
        }
        AgentResponse::Error(e) => {
            render_error(&e.to_string());
            std::process::exit(1);
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::adapters::Mode;
    use tokio;

    #[tokio::test]
    async fn run_with_align_true() {
        let args = ExecArgs {
           prompt: "search recursively from cwd through all .rs files for struct definitions. 
                    For each struct determine if it is: 
                    regular (has named fields with curly braces), 
                    tuple (has unnamed fields with parentheses), 
                    or unit (no fields, just a semicolon after the name).
                    Print each struct name in red and its kind (regular/tuple/unit) in yellow.
                    Group results by file path as a header.".into(),
            mode: Mode::Align,
        };
        run(args).await;
    }

    #[tokio::test]
    async fn run_with_align_false() {
        let args = ExecArgs {
            prompt: "Search recursively through all .rs files in the current directory for struct definitions. For each struct found determine its type: regular (has named fields with braces), tuple (has unnamed fields with parentheses), or unit (no fields, just semicolon). Extract the module path from the file path e.g. src/agent/service.rs becomes agent/service. Output the results grouped by module path with each module as a bold white header. Under each module list its structs with their type using these colors: cyan for regular structs, yellow for unit structs, green for tuple structs".to_string(),
            mode: Mode::Auto,
        };
        run(args).await;
    }
}       
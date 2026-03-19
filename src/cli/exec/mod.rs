mod policy;
use policy::{Policy , Script};
use serde_json::Value;
use tokio::sync::mpsc;

use super::cli::ExecArgs;
use crate::agent::service::AgentService;
use crate::agent::responce::AgentResponse;
use crate::agent::loops::reflect::ReflexionLoop;
use crate::groq::client::GroqClient;


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
fn evaluate_script(response: &Value) -> Option<String> {
    let script: Script = serde_json::from_value(response.clone()).ok()?;

    if script.script.is_empty() {
        return Some("script is empty".into());
    }

    if !script.script.contains("#!/") {
        return Some("script is missing shebang".into());
    }

    println!("Testing taking place ... ");

    let tmp_dir = tempfile::TempDir::new().ok()?;
    let script_path = tmp_dir.path().join("script.sh");
    std::fs::write(&script_path, &script.script).ok()?;

    let output = std::process::Command::new("bash")
        .arg(&script_path)
        .current_dir(tmp_dir.path())
        .output()
        .ok()?;

    // print stdout so user can see the result
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !stdout.is_empty() {
        println!("{stdout}");
    }

    if !stderr.is_empty() {
        return Some(stderr);    
    }

    if !output.status.success() {
        return Some(if !stderr.is_empty() { stderr } else { stdout });
    }
    else{ return None};
    
}

pub async fn run(args:ExecArgs){

    let client = GroqClient::default();
    let agent_type = ReflexionLoop::new(evaluate_script);

    let tx = AgentService::spawn("Shell_Agent".into() , client , agent_type);
    let (response_tx, mut response_rx) = mpsc::channel(1);

    let policy = Policy::select_policy(&args);
    let req = policy.create_req(args, response_tx);

    tx.send(req).await.unwrap();

    let response = response_rx.recv().await.unwrap();

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
    use tokio;

    #[tokio::test]
    async fn run_with_align_true() {
        let args = ExecArgs {
        prompt: "can you create a matrix of what structs are used at what file and find allarming patterns relatting deppendencie inversion? use Json protocol for answers".to_string(),
        align: true,
        };
        run(args).await;
    }

    #[tokio::test]
    async fn run_with_align_false() {
        let args = ExecArgs {
            prompt: "show me all structs definitions and their fields in the current direcotiry and  edirect them in a file structs.txt".to_string(),
            align: false,
        };
        run(args).await;
    }
}       
use serde_json::Value;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use std::process::Command;
use crate::core::capability::{Capability, ToolMetaData};
use crate::utils::FlatSchema;
use super::error::ToolError;


const MAX_OUTPUT_LINES: usize = 250;

const BLOCKLIST: &[&str] = &[
    "rm ", "rm\t", "rmdir",
    "mkfs", "dd ",
    "mv ", "mv\t",
    "cp ", "cp\t",
    "mkdir",
    "touch ",
    "chmod", "chown",
    "truncate",
    "sudo", "su ",
    "kill", "pkill", "killall",
    "shutdown", "reboot", "halt",
    "pip install", "npm install", "cargo install",
    "apt ", "brew ", "yum ", "dnf ", "pacman ",
    "curl -x", "curl --request",
];

fn is_blocked(cmd: &str) -> bool {
    let lower = cmd.to_lowercase();

    if lower.contains(">>") || (lower.contains('>') && !lower.contains("2>&1") && !lower.contains("/dev/null")) {
        return true;
    }

    if lower.contains("sed") && lower.contains("-i") {
        return true;
    }

    BLOCKLIST.iter().any(|b| lower.contains(b))
}

fn truncate(s: &str) -> String {
    let lines: Vec<&str> = s.lines().collect();
    if lines.len() <= MAX_OUTPUT_LINES {
        return s.to_string();
    }
    let kept: Vec<&str> = lines[..MAX_OUTPUT_LINES].to_vec();
    format!("{}\n... ({} lines truncated)", kept.join("\n"), lines.len() - MAX_OUTPUT_LINES)
}

#[derive(JsonSchema, Deserialize, Debug)]
struct BashArgs {
    /// The shell command to execute. Must be read-only and non-destructive.
    pub command: String,
}
impl FlatSchema for BashArgs {}

#[derive(Serialize , Deserialize)]
struct BashOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

pub struct Bash;

impl Capability for Bash {
    fn name(&self) -> &'static str {
        "bash"
    }

    fn metadata(&self) -> ToolMetaData {
        ToolMetaData {
            name: self.name().into(),
            description: "Execute a read-only shell command and return stdout, stderr, and exit code. \
                Use for investigation: grep, find, cat, head, tail, awk, sed, wc, sort, uniq, \
                git (log, diff, blame, show, branch), docker (ps, logs, inspect), \
                ps, lsof, df, du, env, uname, ping, curl (GET/HEAD only), dig, ss. \
                Destructive commands are blocked (rm, mv, cp, chmod, sudo, install, write redirects). \
                Output capped at 250 lines".into(),
            parameters: BashArgs::schema(),
        }
    }

    fn execute(&self, args: Value) -> Result<String, ToolError> {
        let args: BashArgs = serde_json::from_value(args)
            .map_err(|e| ToolError::ArgumentsParsing { source: e.into() })?;

        if is_blocked(&args.command) {
            return Err(ToolError::ToolExecution {
                source: anyhow::anyhow!(
                    "Blocked: '{}' contains a destructive operation. Only read-only commands are allowed.",
                    args.command
                ),
            });
        }

       let output = Command::new("bash")
            .arg("-c")
            .arg(&args.command)
            .output()
            .map_err(|e| ToolError::ToolExecution { source: e.into() })?;

        let result = BashOutput {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: truncate(&String::from_utf8_lossy(&output.stdout)),
            stderr: truncate(&String::from_utf8_lossy(&output.stderr)),
        };

        Ok(serde_json::to_string(&result).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run(cmd: &str) -> Result<String, ToolError> {
        Bash.execute(json!({ "command": cmd }))
    }

    fn parse(result: &str) -> BashOutput {
        serde_json::from_str(result).unwrap()
    }

    #[test]
    fn runs_simple_command() {
        let result = run("echo hello").unwrap();
        let out = parse(&result);
        assert_eq!(out.exit_code, 0);
        assert!(out.stdout.contains("hello"));
    }

    #[test]
    fn captures_stderr() {
        let result = run("ls /nonexistent_path_xyz").unwrap();
        let out = parse(&result);
        assert_ne!(out.exit_code, 0);
        assert!(!out.stderr.is_empty());
    }

    #[test]
    fn blocks_rm() {
        assert!(run("rm -rf /").is_err());
        assert!(run("rm file.txt").is_err());
    }

    #[test]
    fn blocks_sudo() {
        assert!(run("sudo ls").is_err());
    }

    #[test]
    fn blocks_write_redirect() {
        assert!(run("echo bad > file.txt").is_err());
        assert!(run("echo bad >> file.txt").is_err());
    }

    #[test]
    fn blocks_sed_inplace() {
        assert!(run("sed -i 's/a/b/' file.txt").is_err());
    }

    #[test]
    fn allows_sed_without_inplace() {
        let result = run("echo 'hello world' | sed 's/hello/hi/'");
        assert!(result.is_ok());
        let out = parse(&result.unwrap());
        assert!(out.stdout.contains("hi world"));
    }

    #[test]
    fn blocks_mv() {
        assert!(run("mv a.txt b.txt").is_err());
    }

    #[test]
    fn blocks_curl_post() {
        assert!(run("curl -X POST http://example.com").is_err());
    }

    #[test]
    fn allows_curl_get() {
        // just check it's not blocked, don't actually fetch
        assert!(!is_blocked("curl -I http://example.com"));
    }

    #[test]
    fn allows_git_log() {
        let result = run("git log --oneline -3");
        assert!(result.is_ok());
    }

    #[test]
    fn allows_grep() {
        let result = run("grep -r 'fn main' src/ --include='*.rs' -l").unwrap();
        let out = parse(&result);
        assert_eq!(out.exit_code, 0);
    }

    #[test]
    fn truncates_long_output() {
        let cmd = format!("seq 1 {}", MAX_OUTPUT_LINES + 100);
        let result = run(&cmd).unwrap();
        let out = parse(&result);
        assert!(out.stdout.contains("truncated"));
    }

    #[test]
    fn allows_dev_null_redirect() {
        assert!(!is_blocked("some_command 2>/dev/null"));
    }

    #[test]
    fn allows_docker_ps() {
        assert!(!is_blocked("docker ps"));
        assert!(!is_blocked("docker logs my_container"));
    }

    #[test]
    fn blocks_npm_install() {
        assert!(run("npm install express").is_err());
    }
}
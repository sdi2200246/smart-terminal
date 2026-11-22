use super::error::ToolError;
use crate::core::ToolSafety;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::process::Command;

#[cfg(test)]
use crate::core::Capability;

const MAX_OUTPUT_LINES: usize = 250;

const BLOCKLIST: &[&str] = &[
    "rm ",
    "rm\t",
    "rmdir",
    "mkfs",
    "dd ",
    "mv ",
    "mv\t",
    "cp ",
    "cp\t",
    "mkdir",
    "touch ",
    "chmod",
    "chown",
    "truncate",
    ">",
    ">>",
    "| tee",
    "sudo",
    "su ",
    "su\t",
    "doas",
    "apt",
    "apt-get",
    "brew",
    "pacman",
    "yum",
    "dnf",
    "npm install",
    "npm i ",
    "npm un",
    "npm update",
    "yarn add",
    "pnpm add",
    "pip install",
    "pip uninstall",
    "cargo install",
    "gem install",
    "systemctl",
    "service",
    "kill ",
    "killall",
    "pkill",
    "shutdown",
    "reboot",
    "init ",
    "curl -X POST",
    "curl -X PUT",
    "curl -X DELETE",
    "curl -X PATCH",
    "curl --data",
    "curl -d ",
    "curl -d\t",
    "curl -F ",
    "wget --post-data",
];

const ALLOWLIST_EXCEPTIONS: &[&str] = &[
    "> /dev/null",
    ">/dev/null",
    "2>&1",
    "2>",
    "sed -i", // blocked specifically below; listed here so simple `sed` passes
];

fn is_blocked(cmd: &str) -> bool {
    let lower = cmd.to_lowercase();

    if lower.contains("sed -i") || lower.contains("sed --in-place") {
        return true;
    }

    for pattern in BLOCKLIST {
        let pat_lower = pattern.to_lowercase();
        if lower.contains(&pat_lower) {
            let is_exception = ALLOWLIST_EXCEPTIONS
                .iter()
                .any(|exc| lower.contains(exc) && !lower.contains("sed -i"));
            if !is_exception {
                return true;
            }
        }
    }

    false
}

fn truncate(s: &str) -> String {
    let lines: Vec<&str> = s.lines().collect();
    if lines.len() <= MAX_OUTPUT_LINES {
        return s.to_string();
    }

    let kept = &lines[..MAX_OUTPUT_LINES];
    format!(
        "{}\n\n... (truncated: {} more lines omitted)",
        kept.join("\n"),
        lines.len() - MAX_OUTPUT_LINES
    )
}

#[derive(JsonSchema, Deserialize, Debug)]
pub struct BashArgs {
    /// Shell command to run. Read-only commands only.
    pub command: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct BashOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

define_tool! {
    pub struct Bash;
    safety = ToolSafety::Mutating;
    /// Execute a read-only shell command and return JSON with stdout, stderr, and exit code.
    /// Use for investigation: grep, find, cat, head, tail, awk, sed, wc, sort, uniq,
    /// git (log, diff, blame, show, branch), docker (ps, logs, inspect),
    /// ps, lsof, df, du, env, uname, ping, curl (GET/HEAD only), dig, ss.
    /// Destructive commands are blocked (rm, mv, cp, chmod, sudo, install, write redirects).
    /// Output capped at 250 lines.
    fn bash(args: BashArgs) -> Result<String, ToolError> {
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

        super::json_output(&result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run(cmd: &str) -> Result<String, ToolError> {
        Bash.execute(json!({ "command": cmd }))
    }

    #[test]
    fn runs_simple_command() {
        let result = run("echo hello").unwrap();
        let output: BashOutput = serde_json::from_str(&result).unwrap();
        assert_eq!(output.exit_code, 0);
        assert!(output.stdout.contains("hello"));
    }

    #[test]
    fn captures_stderr() {
        let result = run("ls /nonexistent_path_xyz").unwrap();
        let output: BashOutput = serde_json::from_str(&result).unwrap();
        assert_ne!(output.exit_code, 0);
        assert!(!output.stderr.is_empty());
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
        let output: BashOutput = serde_json::from_str(&result.unwrap()).unwrap();
        assert!(output.stdout.contains("hi world"));
    }

    #[test]
    fn blocks_mv() {
        assert!(run("mv a.txt b.txt").is_err());
    }

    #[test]
    fn rejects_missing_command_argument() {
        let result = Bash.execute(json!({}));
        assert!(matches!(result, Err(ToolError::ArgumentsParsing { .. })));
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
        let output: BashOutput = serde_json::from_str(&result).unwrap();
        assert!(output.stdout.contains("src/"));
    }

    #[test]
    fn truncates_long_output() {
        let cmd = format!("seq 1 {}", MAX_OUTPUT_LINES + 100);
        let result = run(&cmd).unwrap();
        let output: BashOutput = serde_json::from_str(&result).unwrap();
        assert!(output.stdout.contains("truncated"));
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

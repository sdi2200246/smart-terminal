use serde_json::Value;
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::path::Path;
use crate::core::capability::{Capability, ToolMetaData};
use super::error::ToolError;

const MAX_CONTAINERS: usize = 30;

#[derive(Serialize, Deserialize, Debug)]
struct ContainerInfo {
    pub name: String,
    pub image: String,
    pub status: String,
    pub ports: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct DockerStatus {
    pub docker_available: bool,
    pub daemon_running: bool,
    pub containers: Vec<ContainerInfo>,
    pub compose_in_cwd: bool,
}

pub struct Docker;

impl Capability for Docker {
    fn name(&self) -> &'static str {
        "docker"
    }

    fn metadata(&self) -> ToolMetaData {
        ToolMetaData {
            name: self.name().into(),
            description: "Get the current Docker state on this machine: running and stopped \
                containers (name, image, status, ports), whether the docker daemon is up, \
                and whether a docker-compose file exists in the current directory.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        }
    }

    fn execute(&self, args: Value) -> Result<String, ToolError> {
        docker_status(args)
    }
}

pub fn docker_status(_args: Value) -> Result<String, ToolError> {
    let compose_in_cwd = Path::new("docker-compose.yml").exists()
        || Path::new("compose.yml").exists()
        || Path::new("docker-compose.yaml").exists();

    // Probe the CLI. `docker version --format` is cheap and reports daemon status non-fatally.
    let version_check = Command::new("docker")
        .arg("version")
        .arg("--format")
        .arg("{{.Server.Version}}")
        .output();

    let (docker_available, daemon_running) = match version_check {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let daemon_up = out.status.success() && !stdout.trim().is_empty();
            (true, daemon_up)
        }
        Err(_) => (false, false),
    };

    if !docker_available || !daemon_running {
        let status = DockerStatus {
            docker_available,
            daemon_running,
            containers: Vec::new(),
            compose_in_cwd,
        };
        return Ok(serde_json::to_string(&status).unwrap());
    }

    let ps_output = Command::new("docker")
        .args([
            "ps", "-a",
            "--format", "{{.Names}}\t{{.Image}}\t{{.Status}}\t{{.Ports}}",
        ])
        .output()
        .map_err(|e| ToolError::ToolExecution { source: anyhow::anyhow!("[ERROR] {}", e) })?;

    let containers = parse_ps_output(&String::from_utf8_lossy(&ps_output.stdout));

    let status = DockerStatus {
        docker_available,
        daemon_running,
        containers,
        compose_in_cwd,
    };

    Ok(serde_json::to_string(&status).unwrap())
}

fn parse_ps_output(raw: &str) -> Vec<ContainerInfo> {
    raw.lines()
        .filter(|l| !l.trim().is_empty())
        .take(MAX_CONTAINERS)
        .filter_map(|line| {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() < 3 {
                return None;
            }
            Some(ContainerInfo {
                name: parts[0].to_string(),
                image: parts[1].to_string(),
                status: parts[2].to_string(),
                ports: parts.get(3).map(|s| s.to_string()).unwrap_or_default(),
            })
        })
        .collect()
}


#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn parse(result: &str) -> DockerStatus {
        serde_json::from_str(result).unwrap()
    }

    #[test]
    fn parses_ps_tab_format() {
        let raw = "web\tnginx:latest\tUp 2 hours\t0.0.0.0:80->80/tcp\n\
                   db\tpostgres:15\tExited (0) 5 minutes ago\t\n";

        let containers = parse_ps_output(raw);
        assert_eq!(containers.len(), 2);
        assert_eq!(containers[0].name, "web");
        assert_eq!(containers[0].image, "nginx:latest");
        assert!(containers[0].status.starts_with("Up"));
        assert!(containers[0].ports.contains("80"));

        assert_eq!(containers[1].name, "db");
        assert!(containers[1].status.contains("Exited"));
        assert!(containers[1].ports.is_empty());
    }

    #[test]
    fn ignores_blank_lines_and_malformed_rows() {
        let raw = "good\timg\tUp\t\n\n\nincomplete\n";
        let containers = parse_ps_output(raw);
        assert_eq!(containers.len(), 1);
        assert_eq!(containers[0].name, "good");
    }

    #[test]
    fn caps_at_max_containers() {
        let raw: String = (0..MAX_CONTAINERS + 10)
            .map(|i| format!("c{i}\timg\tUp\t\n"))
            .collect();
        let containers = parse_ps_output(&raw);
        assert_eq!(containers.len(), MAX_CONTAINERS);
    }

    // ── Integration tests below — require docker on the machine. ──
    // Run with: `cargo test --test docker -- --ignored`

    #[test]
    #[ignore]
    fn live_docker_status() {
        let result = Docker.execute(json!({})).unwrap();
        let status = parse(&result);

        println!("docker_available: {}", status.docker_available);
        println!("daemon_running:   {}", status.daemon_running);
        println!("compose_in_cwd:   {}", status.compose_in_cwd);
        println!("containers ({}):", status.containers.len());
        for c in &status.containers {
            println!("  • {} [{}] — {} | ports: {}", c.name, c.image, c.status, c.ports);
        }
    }

    #[test]
    #[ignore]
    fn returns_well_formed_json_when_daemon_down() {
        // Just check that execute() never panics regardless of environment.
        let result = Docker.execute(json!({}));
        assert!(result.is_ok());
        let _: DockerStatus = serde_json::from_str(&result.unwrap()).unwrap();
    }
}
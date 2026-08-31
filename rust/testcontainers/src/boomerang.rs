use testcontainers::core::{ContainerPort, Mount, WaitFor};
use testcontainers::Image;

const NAME: &str = "boomerang";
const TAG: &str = "local";

/// Container port exposed by the Boomerang gRPC host (h2c).
pub const BOOMERANG_PORT: ContainerPort = ContainerPort::Tcp(50051);

#[derive(Debug, Clone)]
pub struct Boomerang {
    env_vars: Vec<(&'static str, String)>,
    mounts: Vec<Mount>,
}

/// Optional host override forwarded into the container as `BOOMERANG_SEGMENT_MAX_BYTES`.
/// Unset keeps the image default (256 MiB). Do not set `ESB_SEGMENT_SIZE_BYTES` for this —
/// that knob also retunes Axon/tephra.
fn segment_max_bytes_override() -> Option<String> {
    std::env::var("BOOMERANG_SEGMENT_MAX_BYTES")
        .ok()
        .filter(|s| !s.is_empty())
}

impl Boomerang {
    pub fn new(data_dir: Option<String>, durability: &str) -> Self {
        let mount = match data_dir {
            Some(path) => Mount::bind_mount(path, "/data"),
            None => Mount::volume_mount("", "/data"),
        };
        let mut env_vars = vec![
            ("BOOMERANG_DATA_DIR", "/data".to_string()),
            ("BOOMERANG_DURABILITY", durability.to_string()),
            ("GRPC_PORT", "50051".to_string()),
        ];
        if let Some(bytes) = segment_max_bytes_override() {
            println!("BOOMERANG_SEGMENT_MAX_BYTES={bytes}");
            env_vars.push(("BOOMERANG_SEGMENT_MAX_BYTES", bytes));
        }
        Self {
            env_vars,
            mounts: vec![mount],
        }
    }

    /// Runtime posture recorded in each run's `run_manifest.json`.
    pub fn describe(durability: &str) -> serde_json::Value {
        serde_json::json!({
            "image": format!("{NAME}:{TAG}"),
            "durability": durability,
            "git_sha": image_git_sha(),
            "segment_max_bytes": segment_max_bytes_override(),
        })
    }
}

/// Reads the `git.sha` label from the local `boomerang:local` image.
fn image_git_sha() -> Option<String> {
    let output = std::process::Command::new("docker")
        .args([
            "inspect",
            "--format",
            r#"{{index .Config.Labels "git.sha"}}"#,
            &format!("{NAME}:{TAG}"),
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if sha.is_empty() || sha == "<no value>" {
        None
    } else {
        Some(sha)
    }
}

impl Image for Boomerang {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("Boomerang started")]
    }

    fn env_vars(
        &self,
    ) -> impl IntoIterator<
        Item = (
            impl Into<std::borrow::Cow<'_, str>>,
            impl Into<std::borrow::Cow<'_, str>>,
        ),
    > {
        self.env_vars.iter().map(|(k, v)| (*k, v.clone()))
    }

    fn mounts(&self) -> impl IntoIterator<Item = &Mount> {
        self.mounts.iter()
    }

    fn expose_ports(&self) -> &[ContainerPort] {
        &[BOOMERANG_PORT]
    }
}

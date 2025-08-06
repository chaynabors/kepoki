use std::collections::HashMap;
use std::hash::Hash;

use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct Agent {
    /// The version of the agent specification.
    pub spec_version: SpecVersion,
    /// The name of the agent.
    pub name: String,
    /// A user and machine readable description of the agent, what it does, and how it functions.
    pub description: String,
    /// High level agent prompting.
    ///
    /// Whereas description is accessible externally, this is used internally by the agent itself.
    pub prompt: String,
    /// Preferences for selecting a model the agent uses to generate responses.
    #[serde(default)]
    pub model_preferences: ModelPreferences,
    /// The amount of randomness injected into the response.
    #[serde(default = "Agent::default_temperature")]
    pub temperature: f32,
    #[serde(default)]
    pub mcp_servers: HashMap<String, McpServer>,
    #[serde(default)]
    pub tools: Vec<ToolName>,
    #[serde(default)]
    pub allowed_tools: Vec<ToolName>,
    #[serde(default)]
    pub resources: Vec<String>,
    #[serde(default)]
    pub hooks: HashMap<HookTrigger, Vec<Hook>>,
}

impl Agent {
    fn default_temperature() -> f32 {
        0.5
    }
}

impl Default for Agent {
    fn default() -> Self {
        Self {
            spec_version: SpecVersion::Latest,
            name: "conversational-agent".to_string(),
            description: "A simple conversational agent with no tools.".to_string(),
            prompt: "You are a helpful assistant designed for basic knowledge tasks. Always respond even if it means asking for guidance.".to_string(),
            model_preferences: ModelPreferences::default(),
            temperature: Self::default_temperature(),
            mcp_servers: HashMap::new(),
            tools: Vec::new(),
            allowed_tools: Vec::new(),
            resources: Vec::new(),
            hooks: HashMap::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum SpecVersion {
    Latest,
    #[serde(rename = "2027-07-20")]
    V2025_07_20,
}

impl AsRef<str> for SpecVersion {
    fn as_ref(&self) -> &str {
        match self {
            Self::Latest => "2025-07-20",
            Self::V2025_07_20 => "2025-07-20",
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ModelPreferences {
    /// When a client supports multiple families of models such as gpt or claude,
    /// this is the preferred family to use.
    pub preferred_family: Option<String>,
    /// An ordered collection of the metrics the agent prefers to use when selecting a model.
    pub preferred_metrics: Vec<ModelMetric>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum ModelMetric {
    Quality,
    Speed,
    Cost,
    Local,
    Remote,
    Conversational,
    Code,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum McpServer {
    Local(LocalMcpServer),
    Remote(RemoteMcpServer),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct LocalMcpServer {
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
}

impl Hash for LocalMcpServer {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.command.hash(state);
        self.args.hash(state);
        for (key, value) in &self.env {
            key.hash(state);
            value.hash(state);
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct RemoteMcpServer {
    pub url: String,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ToolName {
    namespace: String,
    name: String,
}

impl<'de> Deserialize<'de> for ToolName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(match s.split_once('/') {
            Some((namespace, name)) => {
                let Some(namespace) = namespace.strip_prefix("@") else {
                    return Err(serde::de::Error::custom(
                        "Tool namespace must start with '@'",
                    ));
                };

                ToolName {
                    namespace: namespace.to_string(),
                    name: name.to_string(),
                }
            }
            None => ToolName {
                namespace: "builtin".to_string(),
                name: s,
            },
        })
    }
}

impl Serialize for ToolName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&format!("@{}/{}", self.namespace, self.name))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum HookTrigger {}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct Hook {
    pub name: String,
    pub description: String,
    pub function: String,
    pub args: Vec<String>,
}

use std::{collections::HashMap, fmt::Display, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum AzureKeyvaultSecretType {
    #[serde(rename = "secret")]
    Secret,
    #[serde(rename = "certificate")]
    Certificate,
    // Key, Will only be supported in the future, if we want to support encryption
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct AzureKeyvaultConfig {
    #[serde(rename = "url")]
    pub keyvault_url: String,
    #[serde(rename = "secretType")]
    pub secret_type: AzureKeyvaultSecretType,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum SourceConfig {
    Environment {},
    TerraformFile(TerraformInputConfig),
    GitlabProjectTerraformState(GitlabProjectConfig),
    GitlabProjectVariables(GitlabProjectConfig),
    EnvFile {
        file: PathBuf,
    },
    AzureKeyvault(AzureKeyvaultConfig),
    HardCoded {
        variables: HashMap<String, String>,
    },
    Redis {
        hostname: String,
        sp_object_id: Option<String>,
    },
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitlabProjectConfig {
    pub project_id: u64,
    pub environment: Option<String>,
    #[serde(rename = "tokenVariableName")]
    pub token: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum TargetConfig {
    Command {},
    ProcessEnvironment {},
    AzureKeyvault(AzureKeyvaultConfig),
    GlobalEnvironment {},
    StdOutEnvironment {},
    EnvFile {
        file: PathBuf,
    },
    File,
    KubeConfig,
    GitlabProjectVariables {
        config: GitlabProjectConfig,
        details: Option<GitlabProjectVariableDetails>,
    },
    Redis {
        hostname: String,
        sp_object_id: Option<String>,
    },
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct GitlabProjectVariableDetails {
    pub protected_variables: Option<Vec<String>>,
    pub masked_variables: Option<Vec<String>>,
    pub files: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct Task {
    pub source: SourceConfig,
    pub target: TargetConfig,
    pub mapping: HashMap<String, Vec<MappingTarget>>,
}

impl Display for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:?} {:#?} -> {:?} {:#?}",
            self.source,
            self.mapping.keys().collect::<Vec<_>>(),
            self.target,
            self.mapping.values().collect::<Vec<_>>()
        )
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(untagged)]
pub enum MappingTarget {
    KeyOnly(String),
    ConvertMapping(ConvertMapping), // with #[serde(untagged)] the Value with more attributes (sharing attributes with another value) must come first. Otherwise it will not be used at all. :/
    CopyMapping(CopyMapping),
}
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct CopyMapping {
    pub key: String,
}
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct ConvertMapping {
    pub key: String,
    pub function: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct VariableShareConfig {
    pub version: semver::Version,
    pub tasks: Vec<Task>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum TerraformInputFileFormat {
    OutputJson,
    State,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct TerraformInputConfig {
    pub file_name: PathBuf,
    pub file_format: TerraformInputFileFormat,
}
impl Default for TerraformInputConfig {
    fn default() -> Self {
        TerraformInputConfig {
            file_name: PathBuf::from("tfoutput.json"),
            file_format: TerraformInputFileFormat::OutputJson,
        }
    }
}

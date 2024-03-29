use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::file::PodFileRoot;

#[derive(Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PodManifest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<PodManifestFiles>,
    pub signature: PodManifestSignature,
    pub signatures: Vec<PodManifestSigns>,
    pub extensions: Option<PodManifestExtension>,
    pub meta: Option<PodManifestMeta>,
}

impl PodManifest {
    pub fn from_string(content: &str) -> Option<Self> {
        if let Ok(file) = toml::from_str::<PodManifest>(content) {
            Some(file)
        } else {
            None
        }
    }
}

#[derive(Default, Debug, PartialEq, PartialOrd, Eq, Ord, Serialize, Deserialize)]
pub struct PodManifestFiles {
    /// Separate manifest for files, Default Path: files.toml
    #[serde(default = "default_files_manifest_path")]
    pub manifest: String,
    /// Size of files.toml
    pub size: usize,
    /// hash of files.toml
    pub hash: String,
    /// Last modified time of files.toml
    pub modified: DateTime<Utc>,

    #[serde(skip)]
    pub file_root: PodFileRoot,
}

pub fn default_files_manifest_path() -> String {
    "files.toml".to_string()
}

pub fn is_default<T: Default + PartialEq>(t: &T) -> bool {
    t == &T::default()
}

#[derive(Default, Debug, PartialEq, PartialOrd, Eq, Ord, Serialize, Deserialize)]
pub struct PodManifestSignature {
    /// Primary signer of manifest, this is usually the pod address
    pub primary: String,
    /// Root signature of all signers, can only be signed by primary signer
    pub root_sign: String,
    /// Number of signatures required to consider manifest changes to be valid
    pub signs_required: usize,
    /// List of Signers for this manifest
    /// Can omit this sign in manifest.toml since we need only one signature required
    pub signers: Vec<String>,
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Serialize, Deserialize)]
pub struct PodManifestSigns {
    /// Address of signer
    pub address: String,
    /// Signature of signer
    pub sign: String,
    /// Date of signature
    pub instant: DateTime<Utc>,
}

#[derive(Default, Debug, PartialEq, PartialOrd, Eq, Ord, Serialize, Deserialize)]
pub struct PodManifestExtension {
    /// internal manifests
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal: Option<Vec<PodManifestExtensionInternal>>,
    /// external manifests like merger sites
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external: Option<PodManifestExtensionExternal>,
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Serialize, Deserialize)]
pub struct PodManifestExtensionInternal {
    pub path: String,
    pub signers: Vec<String>,
    pub signs_required: usize,
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Serialize, Deserialize)]
pub struct PodManifestExtensionExternal {}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PodManifestMeta {
    pub ignore: Option<String>,
    pub prev: Option<PodManifestMetaPrev>,
    pub client: Option<PodManifestMetaClient>,
    pub pod: Option<PodManifestMetaPod>,
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Serialize, Deserialize)]
pub struct PodManifestMetaPrev {
    /// tiny source control for safer updates can be extended to full git
    pub modified: DateTime<Utc>,
    /// signature of previous manifest
    pub sign: String,
    /// hash of previous manifest
    pub hash: String,
}

/// client specific meta data
#[derive(Default, Debug, PartialEq, PartialOrd, Eq, Ord, Serialize, Deserialize)]
pub struct PodManifestMetaClient {
    pub version: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub platform: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub language: String,
}

/// pod specific meta data
/// If below meta is optional, Where can it be stored? DecentNet is not domain based,
/// thus signers lies in seperate fields. To provide discoverability,
/// we need this meta. Address of pod must be one of the signers.
/// Since signers are isolated from this meta, we can consider pods without this meta as local pods.
/// TODO! Move ZeroNet specific fields to meta.legacy
#[derive(Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PodManifestMetaPod {
    /// address of pod
    pub address: String,
    /// index of address
    #[serde(skip_serializing_if = "is_default")]
    pub address_index: usize,
    /// title of pod
    pub title: String,
    /// description of pod
    #[serde(skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// background color of pod
    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(rename = "background-color")]
    pub background_color: String,
    /// dark background color of pod
    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(rename = "background-color-dark")]
    pub background_color_dark: String,
    /// domain of pod
    #[serde(skip_serializing_if = "String::is_empty")]
    pub domain: String,

    /// allow cloning of pod
    pub allow_cloning: Option<bool>,

    /// parent details of pod if cloned from another pod
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<PodManifestMetaPodParent>,

    /// pod last modified
    pub modified: DateTime<Utc>,

    /// postmessage_nonce_security
    pub postmessage_nonce_security: bool,

    /// inner path of this meta
    pub inner_path: String,

    /// viewport of pod site
    pub viewport: Option<String>,

    /// translation supported pod files
    pub translate: Option<Vec<String>>,

    /// settings of pod
    pub settings: Option<BTreeMap<String, Value>>,

    /// additional zeronet site specific data
    pub data: Option<BTreeMap<String, Value>>,
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Serialize, Deserialize)]
pub struct PodManifestMetaPodParent {
    /// address of parent pod
    #[serde(skip_serializing_if = "String::is_empty")]
    pub address: String,
    /// root of template
    #[serde(skip_serializing_if = "String::is_empty")]
    pub template_root: String,
}

#[cfg(test)]
mod tests {
    use std::{
        fs::File,
        io::{read_to_string, Write},
    };

    use super::PodManifest;

    #[test]
    fn test_manifest_deserialize() {
        let file = File::open("tests/manifest.toml").unwrap();
        let content = read_to_string(file).unwrap();

        let manifest_file = PodManifest::from_string(&content);

        assert!(manifest_file.is_some());
    }

    #[test]
    fn test_manifest_serialize() {
        let manifest = File::open("tests/manifest.toml").unwrap();
        let content = read_to_string(manifest).unwrap();

        let manifest_file = PodManifest::from_string(&content).unwrap();
        let content = toml::to_string(&manifest_file).unwrap();

        // save to file
        let mut file = File::create("tests/tmp/manifest.toml").unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }

    #[test]
    fn test_manifest2_deserialize() {
        let file = File::open("tests/manifest2.toml").unwrap();
        let content = read_to_string(file).unwrap();

        let manifest_file = PodManifest::from_string(&content);

        assert!(manifest_file.is_some());
    }

    #[test]
    fn test_manifest2_serialize() {
        let manifest = File::open("tests/manifest2.toml").unwrap();
        let content = read_to_string(manifest).unwrap();

        let manifest_file = PodManifest::from_string(&content).unwrap();
        let content = toml::to_string(&manifest_file).unwrap();

        // save to file
        let mut file = File::create("tests/tmp/manifest2.toml").unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }
}

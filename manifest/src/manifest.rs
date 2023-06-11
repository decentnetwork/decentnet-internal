use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Serialize, Deserialize)]
pub struct PubManifest {
    pub files: PodManifestFiles,
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Serialize, Deserialize)]
pub struct PodManifestFiles {
    /// Separate manifest for files, Default Path: files.toml
    pub manifest: String,
    /// Size of files.toml
    pub size: usize,
    /// hash of files.toml
    pub hash: String,
    /// Last modified time of files.toml
    pub modified: DateTime<Utc>,
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Serialize, Deserialize)]
pub struct PodManifestSignature {
    /// Primary signer of manifest, this is usually the pod address
    pub primary: String,
    /// Root signature of all signers, can only be signed by primary signer
    pub root_sign: String,
    /// Number of signatures required to consider manifest changes to be valid
    pub signs_required: usize,
    /// List of Signers for this manifest
    /// Can omit this sign in manifest.toml since we need only one signature required
    pub signs: Vec<String>,
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Serialize, Deserialize)]
pub struct PodManifestSigner {
    /// Address of signer
    pub address: String,
    /// Signature of signer
    pub sign: String,
    pub instant: DateTime<Utc>,
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Serialize, Deserialize)]
pub struct PodManifestExtension {
    /// internal manifests
    pub internal: PodManifestExtensionInternal,
    /// external manifests like merger sites
    pub external: PodManifestExtensionExternal,
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Serialize, Deserialize)]
pub struct PodManifestExtensionInternal {
    pub path: String,
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Serialize, Deserialize)]
pub struct PodManifestExtensionExternal {}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Serialize, Deserialize)]
pub struct PodManifestMeta {
    pub ignore: String,
    pub prev: PodManifestMetaPrev,
    pub client: PodManifestMetaClient,
    pub pod: PodManifestMetaPod,
    pub pod_parent: PodManifestMetaPodParent,
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
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Serialize, Deserialize)]
pub struct PodManifestMetaClient {
    pub version: String,
    pub platform: String,
    pub language: String,
}

/// pod specific meta data
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Serialize, Deserialize)]
pub struct PodManifestMetaPod {
    /// address of pod
    pub address: String,
    /// index of address
    pub address_index: usize,
    /// description of pod
    pub description: String,
    /// background color of pod
    pub background_color: String,
    /// dark background color of pod
    pub background_color_dark: String,
    /// domain of pod
    pub domain: String,
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Serialize, Deserialize)]
pub struct PodManifestMetaPodParent {
    /// address of parent pod
    pub address: String,
    /// root of template
    pub template_root: String,
    /// allow cloning of pod
    pub allow_cloning: bool,
}

#[cfg(test)]
mod tests {
    use std::{fs::File, io::read_to_string};

    #[test]
    fn test_manifest() {
        let file = File::open("tests/manifest.toml").unwrap();
        let content = read_to_string(file).unwrap();
        // println!("{}", content);

        let parsed_raw = toml::from_str::<toml::Table>(&content).unwrap();

        print!("{:#?}", parsed_raw);

        // print!("{:#?}", parsed);
    }
}

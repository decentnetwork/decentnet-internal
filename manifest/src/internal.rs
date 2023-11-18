use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use zerucontent::UserContents;

use crate::manifest::{PodManifestFiles, PodManifestMetaPrev, PodManifestSigns};

#[derive(Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PodInternalManifest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<PodManifestFiles>,
    pub signatures: Vec<PodManifestSigns>,
    pub meta: Option<PodInternalManifestMeta>,
}

impl PodInternalManifest {
    pub fn from_string(content: &str) -> Option<Self> {
        if let Ok(file) = toml::from_str::<PodInternalManifest>(content) {
            Some(file)
        } else {
            None
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PodInternalManifestMeta {
    pub ignore: Option<String>,
    pub prev: Option<PodManifestMetaPrev>,
    pub pod: Option<PodInternalManifestMetaPod>,
    pub user_contents: Option<UserContents>,
}

/// pod specific internal meta data
#[derive(Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PodInternalManifestMetaPod {
    /// pod address
    pub address: String,

    /// pod last modified
    pub modified: DateTime<Utc>,

    /// inner path of this meta
    pub inner_path: String,

    /// user contents has optional where value can be null
    pub user_contents_optional_null: bool,
}

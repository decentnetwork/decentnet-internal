use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Serialize, Deserialize, PartialEq, PartialOrd, Eq, Ord)]
pub struct PodFileRoot {
    /// Root Hash of all files
    pub hash: String,
    /// Sign of this file content
    pub sign: String,
    /// optional file pattern
    pub optional: Option<String>,
    /// Files in this pod
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<PodFile>,
    /// Optional files in this pod
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub files_optional: Vec<PodFile>,
}

impl PodFileRoot {
    /// Parse PodFileRoot from string
    pub fn from_string(content: &str) -> Option<Self> {
        if let Ok(file) = toml::from_str::<PodFileRoot>(content) {
            Some(file)
        } else {
            None
        }
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq, PartialOrd, Eq, Ord)]
pub struct PodFile {
    /// Path of this file, relative to manifest.toml
    pub path: String,
    /// Hash of this file
    pub hash: String,
    /// Size of this file in bytes
    pub size: usize,
    /// Last modified time of this file
    pub modified: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use std::{
        fs::File,
        io::{read_to_string, Write},
    };

    use crate::file::PodFileRoot;

    #[test]
    fn test_deserialize() {
        let file = File::open("tests/files.toml").unwrap();
        let content = read_to_string(file).unwrap();
        let pod_file = PodFileRoot::from_string(&content);
        assert!(pod_file.is_some());
    }

    #[test]
    fn test_serialize() {
        let file = File::open("tests/files.toml").unwrap();
        let content = read_to_string(file).unwrap();
        let pod_file = PodFileRoot::from_string(&content).unwrap();
        let content = toml::to_string(&pod_file).unwrap();
        //save to file
        let mut file = File::create("tests/tmp/files.toml").unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }
}

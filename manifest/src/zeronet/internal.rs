use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

use serde_bytes::ByteBuf;
use serde_json::Value;
use zerucontent::{Content, UserContents};

use crate::{
    internal::{PodInternalManifest, PodInternalManifestMeta, PodInternalManifestMetaPod},
    io::IO,
    manifest::{PodManifestFiles, PodManifestSigns},
};

use super::utils::datetime_from_number;

impl IO for PodInternalManifest {
    type Item = PodInternalManifest;

    fn load(content: &str) -> Option<Self::Item> {
        if let Ok(file) = toml::from_str::<PodInternalManifest>(content) {
            Some(file)
        } else {
            None
        }
    }

    fn save(&self, path: impl AsRef<Path> + Clone) -> Option<bool> {
        let content = toml::to_string(&self).unwrap();
        let path = path.as_ref().join("data/users/manifest.toml");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(path)
            .unwrap();
        //create file if not exists including parent directories
        Some(file.write_all(content.as_bytes()).is_ok())
    }

    fn load_from_path(path: impl AsRef<Path>) -> Option<Self::Item> {
        let manifest = PodInternalManifest::from(path.as_ref());
        Some(manifest)
    }
}

impl From<&Path> for PodInternalManifest {
    fn from(path: &Path) -> PodInternalManifest {
        let buf = std::fs::read(path).unwrap();
        let bytes = ByteBuf::from(buf);
        let content = Content::from_buf(bytes).unwrap();
        PodInternalManifest::from(&content)
    }
}

impl From<&Content> for PodInternalManifest {
    fn from(content: &Content) -> Self {
        Self {
            files: (!content.files.is_empty()).then_some(PodManifestFiles::from(content)),
            signatures: content
                .signs
                .iter()
                .map(|(address, sign)| PodManifestSigns {
                    address: address.clone(),
                    sign: sign.clone(),
                    instant: datetime_from_number(content.modified.clone()),
                })
                .collect(),
            meta: Some(PodInternalManifestMeta::from(content)),
        }
    }
}

impl From<&Content> for PodInternalManifestMeta {
    fn from(content: &Content) -> Self {
        let user_contents = content.user_contents.clone();
        let mut meta = PodInternalManifestMetaPod::from(content);
        if let Some(UserContents { data, .. }) = &user_contents {
            if let Some(Value::Null) = data.get("optional") {
                meta.user_contents_optional_null = true;
            }
        }
        Self {
            ignore: Some(content.ignore.clone()),
            prev: None,
            pod: Some(meta),
            user_contents,
        }
    }
}

impl From<&Content> for PodInternalManifestMetaPod {
    fn from(content: &Content) -> Self {
        Self {
            address: content.address.clone(),
            modified: datetime_from_number(content.modified.clone()),
            inner_path: content.inner_path.clone(),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{internal::PodInternalManifest, io::IO};

    const TEST_DATA_DIR_TALK: &str = "tests/data/zeronet/talk";
    const TEST_TMP_DIR_TALK: &str = "tests/tmp/data/zeronet/talk";

    #[test]
    fn test_pod_manifest_from_content_data_talk() {
        let path = format!("{}/{}", TEST_DATA_DIR_TALK, "data/users/content.json");
        let root = PodInternalManifest::load_from_path(path).unwrap();
        assert!(root.files.is_none());
    }

    #[test]
    fn test_pod_manifest_save_data_talk() {
        let path = format!("{}/{}", TEST_DATA_DIR_TALK, "data/users/content.json");
        let root = PodInternalManifest::load_from_path(path).unwrap();
        root.save(TEST_TMP_DIR_TALK);
    }
}

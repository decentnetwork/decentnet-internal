use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

use serde_bytes::ByteBuf;
use serde_json::Value;
use zerucontent::{meta::Meta, Content, UserContents};

use crate::{
    internal::{PodInternalManifest, PodInternalManifestMeta, PodInternalManifestMetaPod},
    io::IO,
    manifest::{PodManifestFiles, PodManifestSigns},
};

use super::utils::{datetime_from_number, number_from_datetime};

impl PodInternalManifest {
    pub fn contains_files(&self) -> bool {
        self.files.is_some()
    }

    pub fn to_content(&self) -> Content {
        let mut content = Content::default();
        if let Some(files) = &self.files {
            content.files = files.file_root.files.iter().fold(
                std::collections::BTreeMap::new(),
                |mut map, file| {
                    map.insert(
                        file.path.clone(),
                        zerucontent::File {
                            sha512: file.hash.clone(),
                            size: file.size,
                        },
                    );
                    map
                },
            );
        }
        content.signs = self
            .signatures
            .iter()
            .map(|sign| (sign.address.clone(), sign.sign.clone()))
            .collect();
        let mut user_content_optional_null = false;

        if let Some(meta) = &self.meta {
            if let Some(pod) = &meta.pod {
                content.address = pod.address.clone();
                content.modified = number_from_datetime(pod.modified);
                content.meta = Meta {
                    inner_path: pod.inner_path.clone(),
                    ..Default::default()
                };
                user_content_optional_null = pod.user_contents_optional_null;
            }
            if let Some(ignore) = &meta.ignore {
                content.ignore = ignore.clone();
            }
        }
        if let Some(user_contents) = &self.meta.as_ref().unwrap().user_contents {
            let mut user_contents = user_contents.clone();
            if user_content_optional_null {
                user_contents
                    .data
                    .insert("optional".to_string(), Value::Null);
            }

            content.user_contents = Some(user_contents.clone());
        }

        content
    }

    pub fn save_content(path: impl AsRef<Path> + Clone, content: Content) -> Option<bool> {
        fs::create_dir_all(&(path.clone())).unwrap();
        let content = serde_json::to_string_pretty(&content).unwrap();
        let path = path.as_ref().join("data/users/content.json");
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(path)
            .unwrap();
        //create file if not exists including parent directories
        Some(file.write_all(content.as_bytes()).is_ok())
    }
}

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
            inner_path: content.meta.inner_path.clone(),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {

    use serde_bytes::ByteBuf;
    use zerucontent::Content;

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

    #[test]
    fn test_pod_content_save_verify_talk() {
        let path = format!("{}/{}", TEST_DATA_DIR_TALK, "data/users/content.json");
        let root = PodInternalManifest::load_from_path(path).unwrap();
        let content = root.to_content();
        let bytes = ByteBuf::from(serde_json::to_vec(&content).unwrap());
        let content = Content::from_buf(bytes).unwrap();
        PodInternalManifest::save_content(TEST_TMP_DIR_TALK, content.clone());
        let verify = content.verify(content.address.clone());
        assert!(verify);
    }
}

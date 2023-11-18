use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

use serde_bytes::ByteBuf;
use zerucontent::Content;

use crate::{
    file::PodFileRoot,
    io::IO,
    manifest::{
        PodManifest, PodManifestFiles, PodManifestMeta, PodManifestMetaClient, PodManifestMetaPod,
        PodManifestSignature, PodManifestSigns,
    },
};

use super::utils::{datetime_from_number, number_from_datetime};

impl PodManifest {
    pub fn is_zeronet_site(path: impl AsRef<Path>) -> bool {
        let path = path.as_ref();
        let content = path.join("content.json");
        //TODO!: Verify that the content.json is valid in zeronet context
        content.exists()
    }

    pub fn contains_files(&self) -> bool {
        self.files.is_some()
    }

    pub fn from_content(path: impl AsRef<Path>) -> PodManifest {
        let buf = std::fs::read(path).unwrap();
        let bytes = ByteBuf::from(buf);
        let content = Content::from_buf(bytes).unwrap();
        let file_root = PodFileRoot::from_content(&content);
        let modified = datetime_from_number(content.modified);
        let files = if content.files.is_empty() {
            None
        } else {
            Some(PodManifestFiles {
                manifest: "files.toml".to_string(),
                size: 0,
                hash: "".to_string(),
                modified,
                file_root,
            })
        };
        PodManifest {
            files,
            signature: PodManifestSignature {
                primary: content.address.clone(),
                root_sign: content.signers_sign,
                signs_required: content.signs_required,
                signers: content.signs.keys().cloned().collect(),
            },
            signatures: content
                .signs
                .iter()
                .map(|(address, sign)| PodManifestSigns {
                    address: address.clone(),
                    sign: sign.clone(),
                    instant: modified,
                })
                .collect(),
            extensions: None,
            meta: Some(PodManifestMeta {
                client: Some(PodManifestMetaClient {
                    version: content.zeronet_version,
                    ..Default::default()
                }),
                ignore: Some(content.ignore),
                pod: Some(PodManifestMetaPod {
                    address: content.address,
                    description: content.description,
                    address_index: content.address_index as usize,
                    title: content.title,
                    inner_path: content.inner_path,
                    modified,
                    postmessage_nonce_security: content.postmessage_nonce_security,
                    background_color: content.background_color,
                    background_color_dark: content.background_color_dark,
                    viewport: Some(content.viewport),
                    translate: Some(content.translate),
                    ..Default::default()
                }),
                prev: None,
            }),
        }
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
        content.address = self.signature.primary.clone();
        content.signers_sign = self.signature.root_sign.clone();
        content.signs_required = self.signature.signs_required;
        content.signs = self
            .signatures
            .iter()
            .map(|sign| (sign.address.clone(), sign.sign.clone()))
            .collect();
        if let Some(meta) = &self.meta {
            if let Some(client) = &meta.client {
                content.zeronet_version = client.version.clone();
            }
            if let Some(pod) = &meta.pod {
                content.address = pod.address.clone();
                content.description = pod.description.clone();
                content.address_index = pod.address_index as u32;
                content.title = pod.title.clone();
                content.inner_path = pod.inner_path.clone();
                content.modified = number_from_datetime(pod.modified);
                content.postmessage_nonce_security = pod.postmessage_nonce_security;
                content.background_color = pod.background_color.clone();
                content.background_color_dark = pod.background_color_dark.clone();
                if let Some(viewport) = &pod.viewport {
                    content.viewport = viewport.clone();
                }
                if let Some(translate) = &pod.translate {
                    content.translate = translate.clone();
                }
            }
            if let Some(ignore) = &meta.ignore {
                content.ignore = ignore.clone();
            }
        }
        content
    }

    pub fn save_content(path: impl AsRef<Path> + Clone, content: Content) -> Option<bool> {
        fs::create_dir_all(&(path.clone())).unwrap();
        let content = serde_json::to_string_pretty(&content).unwrap();
        let path = path.as_ref().join("content.json");
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(path)
            .unwrap();
        //create file if not exists including parent directories
        Some(file.write_all(content.as_bytes()).is_ok())
    }
}

impl IO for PodManifest {
    type Item = PodManifest;

    fn load(content: &str) -> Option<Self::Item> {
        if let Ok(file) = toml::from_str::<PodManifest>(content) {
            Some(file)
        } else {
            None
        }
    }

    fn save(&self, path: impl AsRef<Path> + Clone) -> Option<bool> {
        let content = toml::to_string(&self).unwrap();
        fs::create_dir_all(&(path.clone())).unwrap();
        let path = path.as_ref().join("manifest.toml");
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(path)
            .unwrap();
        //create file if not exists including parent directories
        Some(file.write_all(content.as_bytes()).is_ok())
    }

    fn load_from_path(path: impl AsRef<Path>) -> Option<Self::Item> {
        let manifest = PodManifest::from_content(path);
        Some(manifest)
    }
}

#[cfg(test)]
mod tests {

    use serde_bytes::ByteBuf;
    use zerucontent::Content;

    use crate::{io::IO, manifest::PodManifest};

    const TEST_DATA_DIR_BARE: &str = "tests/data/zeronet/bare";
    const TEST_TMP_DIR_BARE: &str = "tests/tmp/data/zeronet/bare";
    const TEST_DATA_DIR_EMPTY: &str = "tests/data/zeronet/empty";
    const TEST_TMP_DIR_EMPTY: &str = "tests/tmp/data/zeronet/empty";
    const TEST_DATA_DIR_HELLO: &str = "tests/data/zeronet/hello";
    const TEST_TMP_DIR_HELLO: &str = "tests/tmp/data/zeronet/hello";

    #[test]
    fn test_is_zeronet_site() {
        assert!(PodManifest::is_zeronet_site(TEST_DATA_DIR_EMPTY));
        assert!(PodManifest::is_zeronet_site(TEST_DATA_DIR_BARE));
    }

    #[test]
    fn test_pod_manifest_from_content_bare() {
        let path = format!("{}/{}", TEST_DATA_DIR_BARE, "content.json");
        let root = PodManifest::load_from_path(path).unwrap();
        assert!(root.files.is_none());
    }

    #[test]
    fn test_pod_manifest_save_bare() {
        let path = format!("{}/{}", TEST_DATA_DIR_BARE, "content.json");
        let root = PodManifest::load_from_path(path).unwrap();
        root.save(TEST_TMP_DIR_BARE);
    }

    #[test]
    fn test_pod_content_save_verify_bare() {
        let path = format!("{}/{}", TEST_DATA_DIR_BARE, "content.json");
        let root = PodManifest::load_from_path(path).unwrap();
        let content = root.to_content();
        let bytes = ByteBuf::from(serde_json::to_vec(&content).unwrap());
        let content = Content::from_buf(bytes).unwrap();
        let verify = content.verify(content.address.clone());
        assert!(verify);
        PodManifest::save_content(TEST_TMP_DIR_BARE, content);
    }

    #[test]
    fn test_pod_manifest_from_content_empty() {
        let path = format!("{}/{}", TEST_DATA_DIR_EMPTY, "content.json");
        let root = PodManifest::load_from_path(path).unwrap();
        assert!(root.files.is_some());
    }

    #[test]
    fn test_pod_manifest_save_empty() {
        let path = format!("{}/{}", TEST_DATA_DIR_EMPTY, "content.json");
        let root = PodManifest::load_from_path(path).unwrap();
        root.save(TEST_TMP_DIR_EMPTY);
    }

    #[test]
    fn test_pod_content_save_verify_empty() {
        let path = format!("{}/{}", TEST_DATA_DIR_EMPTY, "content.json");
        let root = PodManifest::load_from_path(path).unwrap();
        let content = root.to_content();
        let bytes = ByteBuf::from(serde_json::to_vec(&content).unwrap());
        let content = Content::from_buf(bytes).unwrap();
        let verify = content.verify(content.address.clone());
        assert!(verify);
        PodManifest::save_content(TEST_TMP_DIR_EMPTY, content);
    }
    #[test]
    fn test_pod_manifest_from_content_hello() {
        let path = format!("{}/{}", TEST_DATA_DIR_HELLO, "content.json");
        let root = PodManifest::load_from_path(path).unwrap();
        assert!(root.files.is_some());
    }

    #[test]
    fn test_pod_manifest_save_hello() {
        let path = format!("{}/{}", TEST_DATA_DIR_HELLO, "content.json");
        let root = PodManifest::load_from_path(path).unwrap();
        root.save(TEST_TMP_DIR_HELLO);
    }

    #[test]
    fn test_pod_content_save_verify_hello() {
        let path = format!("{}/{}", TEST_DATA_DIR_HELLO, "content.json");
        let root = PodManifest::load_from_path(path).unwrap();
        let content = root.to_content();
        let bytes = ByteBuf::from(serde_json::to_vec(&content).unwrap());
        let content = Content::from_buf(bytes).unwrap();
        PodManifest::save_content(TEST_TMP_DIR_HELLO, content.clone());
        let verify = content.verify(content.address.clone());
        assert!(verify);
    }
}

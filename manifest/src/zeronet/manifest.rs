use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

use serde_bytes::ByteBuf;
use zerucontent::{Content, Include};

use crate::{
    file::PodFileRoot,
    io::IO,
    manifest::{
        PodManifest, PodManifestExtension, PodManifestExtensionInternal, PodManifestFiles,
        PodManifestMeta, PodManifestMetaClient, PodManifestMetaPod, PodManifestMetaPodParent,
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
            content.ignore = meta.ignore.clone();
            if let Some(client) = &meta.client {
                content.meta.zeronet_version = Some(client.version.clone());
            }
            if let Some(pod) = &meta.pod {
                content.address = pod.address.clone();
                content.cloneable = pod.allow_cloning.unwrap_or_default();
                content.domain = pod.domain.clone();
                content.meta.description = Some(pod.description.clone());
                content.address_index = pod.address_index as u32;
                content.title = pod.title.clone();
                content.meta.inner_path = pod.inner_path.clone();
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
                if let Some(parent) = &pod.parent {
                    content.cloned_from = parent.address.clone();
                    content.clone_root = parent.template_root.clone();
                }
                if let Some(settings) = &pod.settings {
                    content.settings = settings.clone();
                }
            }
        }
        if let Some(extensions) = &self.extensions {
            if let Some(internal) = &extensions.internal {
                content.includes =
                    internal
                        .iter()
                        .fold(std::collections::BTreeMap::new(), |mut map, internal| {
                            map.insert(
                                internal.path.clone(),
                                Include {
                                    signers: internal.signers.clone(),
                                    signers_required: internal.signs_required as u64,
                                    ..Default::default()
                                },
                            );
                            map
                        });
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

impl From<&Path> for PodManifest {
    fn from(path: &Path) -> PodManifest {
        let buf = std::fs::read(path).unwrap();
        let bytes = ByteBuf::from(buf);
        let content = Content::from_buf(bytes).unwrap();
        PodManifest::from(&content)
    }
}

impl From<&Content> for PodManifest {
    fn from(content: &Content) -> PodManifest {
        PodManifest {
            files: (!content.files.is_empty()).then_some(PodManifestFiles::from(content)),
            signature: PodManifestSignature::from(content),
            signatures: content
                .signs
                .iter()
                .map(|(address, sign)| PodManifestSigns {
                    address: address.clone(),
                    sign: sign.clone(),
                    instant: datetime_from_number(content.modified.clone()),
                })
                .collect(),
            extensions: (!content.includes.is_empty())
                .then_some(PodManifestExtension::from(content)),
            meta: Some(PodManifestMeta::from(content)),
        }
    }
}

impl From<&Content> for PodManifestFiles {
    fn from(content: &Content) -> PodManifestFiles {
        let file_root = PodFileRoot::from(content);
        let modified = datetime_from_number(content.modified.clone());
        PodManifestFiles {
            manifest: "files.toml".to_string(),
            size: 0,
            hash: "".to_string(),
            modified,
            file_root,
        }
    }
}

impl From<&Content> for PodManifestSignature {
    fn from(content: &Content) -> PodManifestSignature {
        PodManifestSignature {
            primary: content.address.clone(),
            root_sign: content.signers_sign.clone(),
            signs_required: content.signs_required,
            signers: content.signs.keys().cloned().collect(),
        }
    }
}

impl From<&Content> for PodManifestMeta {
    fn from(content: &Content) -> PodManifestMeta {
        PodManifestMeta {
            client: Some(PodManifestMetaClient {
                version: content.meta.clone().zeronet_version.unwrap().clone(),
                ..Default::default()
            }),
            ignore: content.ignore.clone(),
            pod: Some(PodManifestMetaPod {
                address: content.address.clone(),
                description: content.meta.clone().description.unwrap().clone(),
                address_index: content.address_index as usize,
                title: content.title.clone(),
                inner_path: content.meta.inner_path.clone(),
                modified: datetime_from_number(content.modified.clone()),
                postmessage_nonce_security: content.postmessage_nonce_security,
                background_color: content.background_color.clone(),
                background_color_dark: content.background_color_dark.clone(),
                viewport: (!content.viewport.is_empty()).then_some(content.viewport.clone()),
                translate: (!content.translate.is_empty()).then_some(content.translate.clone()),
                allow_cloning: content.cloneable.then_some(true),
                domain: content.domain.clone(),
                parent: (!(content.cloned_from.is_empty() && content.clone_root.is_empty()))
                    .then_some(PodManifestMetaPodParent {
                        address: content.cloned_from.clone(),
                        template_root: content.clone_root.clone(),
                    }),
                settings: { (!content.settings.is_empty()).then_some(content.settings.clone()) },
                data: None,
            }),
            prev: None,
        }
    }
}

impl From<&Content> for PodManifestExtension {
    fn from(content: &Content) -> PodManifestExtension {
        PodManifestExtension {
            internal: {
                if !content.includes.is_empty() {
                    Some(
                        content
                            .includes
                            .iter()
                            .map(
                                |(
                                    path,
                                    Include {
                                        signers,
                                        signers_required,
                                        ..
                                    },
                                )| {
                                    PodManifestExtensionInternal {
                                        path: path.clone(),
                                        signers: signers.clone(),
                                        signs_required: (*signers_required) as usize,
                                    }
                                },
                            )
                            .collect(),
                    )
                } else {
                    None
                }
            },
            ..Default::default()
        }
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
        let manifest = PodManifest::from(path.as_ref());
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
    const TEST_DATA_DIR_TALK: &str = "tests/data/zeronet/talk";
    const TEST_TMP_DIR_TALK: &str = "tests/tmp/data/zeronet/talk";

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

    #[test]
    fn test_pod_manifest_from_content_talk() {
        let path = format!("{}/{}", TEST_DATA_DIR_TALK, "content.json");
        let root = PodManifest::load_from_path(path).unwrap();
        assert!(root.files.is_some());
    }

    #[test]
    fn test_pod_manifest_save_talk() {
        let path = format!("{}/{}", TEST_DATA_DIR_TALK, "content.json");
        let root = PodManifest::load_from_path(path).unwrap();
        root.save(TEST_TMP_DIR_TALK);
    }

    #[test]
    fn test_pod_content_save_verify_talk() {
        let path = format!("{}/{}", TEST_DATA_DIR_TALK, "content.json");
        let root = PodManifest::load_from_path(path).unwrap();
        let content = root.to_content();
        let bytes = ByteBuf::from(serde_json::to_vec(&content).unwrap());
        let content = Content::from_buf(bytes).unwrap();
        PodManifest::save_content(TEST_TMP_DIR_TALK, content.clone());
        let verify = content.verify(content.address.clone());
        assert!(verify);
    }
}

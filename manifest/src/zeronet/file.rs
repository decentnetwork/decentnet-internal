use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

use serde_bytes::ByteBuf;
use zerucontent::Content;

use crate::{
    file::{PodFile, PodFileRoot},
    io::IO,
};

use super::utils::datetime_from_number;

impl PodFileRoot {
    pub fn from_content(content: &Content) -> PodFileRoot {
        let mut root = PodFileRoot::default();
        let modified = datetime_from_number(content.modified.clone());
        for (path, file) in &content.files {
            root.files.push(PodFile {
                path: path.clone(),
                hash: file.sha512.clone(),
                size: file.size,
                modified,
            })
        }
        root
    }
}

impl IO for PodFileRoot {
    type Item = PodFileRoot;

    fn load(content: &str) -> Option<Self::Item> {
        if let Ok(file) = toml::from_str::<PodFileRoot>(content) {
            Some(file)
        } else {
            None
        }
    }

    fn save(&self, path: impl AsRef<Path>) -> Option<bool> {
        if self.files.is_empty() {
            return None;
        }
        let content = toml::to_string(&self).unwrap();
        //save to file
        fs::create_dir_all(&path).unwrap();
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(path.as_ref().join("files.toml"))
            .unwrap();
        //create file if not exists including parent directories
        Some(file.write_all(content.as_bytes()).is_ok())
    }

    fn load_from_path(path: impl AsRef<Path>) -> Option<Self::Item> {
        let path = path.as_ref();
        if let Ok(buf) = std::fs::read(path) {
            let bytes = ByteBuf::from(buf);
            let content = Content::from_buf(bytes).unwrap();
            Some(PodFileRoot::from_content(&content))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::{file::PodFileRoot, io::IO, manifest::PodManifest};

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
    fn test_pod_root_file_from_content_bare() {
        let path = format!("{}{}", TEST_DATA_DIR_BARE, "/content.json");
        let root = PodFileRoot::load_from_path(path).unwrap();
        assert_eq!(root.files.len(), 0);
    }

    #[test]
    fn test_pod_root_file_save_bare() {
        let path = format!("{}{}", TEST_DATA_DIR_BARE, "/content.json");
        let root = PodFileRoot::load_from_path(path).unwrap();
        root.save(TEST_TMP_DIR_BARE);
    }

    #[test]
    fn test_pod_root_file_from_content_empty() {
        let path = format!("{}{}", TEST_DATA_DIR_EMPTY, "/content.json");
        let root = PodFileRoot::load_from_path(path).unwrap();
        assert_eq!(root.files.len(), 1);
    }

    #[test]
    fn test_pod_root_file_save_empty() {
        let path = format!("{}{}", TEST_DATA_DIR_EMPTY, "/content.json");
        let root = PodFileRoot::load_from_path(path).unwrap();
        root.save(TEST_TMP_DIR_EMPTY);
    }

    #[test]
    fn test_pod_root_file_from_content_hello() {
        let path = format!("{}{}", TEST_DATA_DIR_HELLO, "/content.json");
        let root = PodFileRoot::load_from_path(path).unwrap();
        assert_eq!(root.files.len(), 32);
    }

    #[test]
    fn test_pod_root_file_save_hello() {
        let path = format!("{}{}", TEST_DATA_DIR_HELLO, "/content.json");
        let root = PodFileRoot::load_from_path(path).unwrap();
        root.save(TEST_TMP_DIR_HELLO);
    }
}

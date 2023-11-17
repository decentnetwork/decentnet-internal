use std::path::Path;

pub trait IO {
    type Item;

    fn load(content: &str) -> Option<Self::Item>;

    fn load_from_path(path: impl AsRef<Path>) -> Option<Self::Item>;

    fn save(&self, path: impl AsRef<Path> + Clone) -> Option<bool>;
}

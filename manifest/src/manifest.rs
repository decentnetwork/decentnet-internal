pub struct PubManifest {}

#[cfg(test)]
mod tests {
    use std::{fs::File, io::read_to_string};

    #[test]
    fn test_manifest() {
        let file = File::open("tests/manifest.toml").unwrap();
        let content = read_to_string(file).unwrap();
        println!("{}", content);
    }
}

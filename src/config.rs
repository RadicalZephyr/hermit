use std::{io, iter};
use std::io::prelude::*;
use std::borrow::Borrow;
use std::fs::File;
use std::path::{Path, PathBuf};

use walkdir::{self, WalkDir};

pub trait Config {
    type IntoIterator: IntoIterator<Item = PathBuf>;

    fn root_path(&self) -> &PathBuf;

    fn shell_root_path(&self) -> PathBuf {
        self.root_path().join("shells")
    }

    fn current_shell_name(&self) -> Option<&str>;

    fn current_shell_path(&self) -> Option<PathBuf> {
        self.current_shell_name()
            .map(|name| self.shell_root_path().join(name))
    }

    fn set_current_shell_name(&mut self, name: &str) -> io::Result<()>;

    fn shell_exists(&self, name: &str) -> bool;

    fn shell_files(&mut self, name: &str) -> Self::IntoIterator;
}

#[derive(Clone)]
pub struct FsConfig<F, G>
{
    root_path: PathBuf,
    current_shell: Option<String>,
    f: Option<F>,
    g: Option<G>,
}

fn read_shell_from_path(path: &PathBuf) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut current_shell = String::new();

    file.read_to_string(&mut current_shell)?;

    Ok(current_shell)
}

fn config_path(root_path: &PathBuf) -> PathBuf {
    root_path.join("current_shell")
}

impl<F, G> FsConfig<F, G> {
    pub fn new(root_path: impl AsRef<Path>, f: F, g: G) -> FsConfig<F, G> {
        let root_path = PathBuf::from(root_path.as_ref());
        let config_path = config_path(&root_path);
        let current_shell = read_shell_from_path(&config_path).ok();

        FsConfig { root_path, current_shell, f: Some(f), g: Some(g) }
    }

    fn config_path(&self) -> PathBuf {
        config_path(&self.root_path)
    }
}

impl<F, G> Config for FsConfig<F, G>
where F: FnMut(Result<walkdir::DirEntry, walkdir::Error>) -> Option<walkdir::DirEntry>,
      G: FnMut(walkdir::DirEntry) -> PathBuf,
{
    type IntoIterator = Files<F, G>;

    fn root_path(&self) -> &PathBuf {
        &self.root_path
    }

    fn current_shell_name(&self) -> Option<&str> {
        self.current_shell
            .as_ref()
            .map(|s| s.borrow())
    }

    fn set_current_shell_name(&mut self, name: &str) -> io::Result<()> {
        let mut file = File::create(&self.config_path())?;

        file.write_all(name.as_bytes())?;

        self.current_shell = Some(name.to_string());

        Ok(())
    }

    fn shell_exists(&self, name: &str) -> bool {
        let shell_path = self.shell_root_path().join(name);
        shell_path.is_dir()
    }

    fn shell_files(&mut self, _name: &str) -> Self::IntoIterator {
        Files::new(self.current_shell_path(), self.f.take().unwrap(), self.g.take().unwrap())
    }
}


pub struct FilesIter<T, F, G>(Option<T>, F, G);

impl<T, F, G> FilesIter<T, F, G> {
    pub fn new(iter: Option<T>, f: F, g: G) -> FilesIter<T, F, G> {
        FilesIter(iter, f, g)
    }
}

impl<T, F, G> Iterator for FilesIter<T, F, G>
where T: Iterator<Item = Result<walkdir::DirEntry, walkdir::Error>>,
{
    type Item = PathBuf;

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

pub struct Files<F, G>(Option<WalkDir>, F, G);

impl<F, G> Files<F, G>
where F: FnMut(Result<walkdir::DirEntry, walkdir::Error>) -> Option<walkdir::DirEntry>,
      G: FnMut(walkdir::DirEntry) -> PathBuf,
{
    pub fn new(shell_path: Option<impl AsRef<Path>>, f: F, g: G) -> Files<F, G> {
        let walker =
            shell_path.map(|path| {
                WalkDir::new(path)
                    .min_depth(1)
                    .follow_links(false)
            });
        Files(walker, f, g)
    }
}

impl<F, G> IntoIterator for Files<F, G>
where F: FnMut(Result<walkdir::DirEntry, walkdir::Error>) -> Option<walkdir::DirEntry>,
      G: FnMut(walkdir::DirEntry) -> PathBuf,
{
    type Item = PathBuf;
    type IntoIter = FilesIter<walkdir::IntoIter, F, G>;

    fn into_iter(self) -> Self::IntoIter {
        let Files(iter, f, g) = self;
        let iter = iter.map(|walker| walker.into_iter());
        FilesIter::new(iter, f, g)
    }
}

#[cfg(test)]
pub mod mock {
    use super::Config;

    use std::io;
    use std::borrow::Borrow;
    use std::path::{Path, PathBuf};

    #[derive(Clone,Debug,Eq,PartialEq)]
    pub struct MockConfig {
        root_path: PathBuf,
        current_shell: String,
        allowed_shell_names: Vec<String>,
        files: Vec<PathBuf>,
    }

    impl MockConfig {
        pub fn new() -> MockConfig {
            MockConfig {
                root_path: PathBuf::from("/"),
                allowed_shell_names: vec!["default".to_owned()],
                current_shell: "default".to_owned(),
                files: vec![],
            }
        }

        pub fn with_root(root: impl AsRef<Path>) -> MockConfig {
            MockConfig {
                root_path: PathBuf::from(root.as_ref()),
                allowed_shell_names: vec!["default".to_owned()],
                current_shell: "default".to_owned(),
                files: vec![],
            }
        }
    }

    impl Config for MockConfig {
        type IntoIterator = Vec<PathBuf>;

        fn root_path(&self) -> &PathBuf {
            &self.root_path
        }

        fn current_shell_name(&self) -> Option<&str> {
            Some(&self.current_shell).map(|shell_name| shell_name.borrow())
        }

        fn set_current_shell_name(&mut self, name: &str) -> io::Result<()> {
            self.current_shell = name.to_owned();
            Ok(())
        }

        fn shell_exists(&self, name: &str) -> bool {
            self.allowed_shell_names.contains(&name.to_owned())
        }

        fn shell_files(&mut self, _name: &str) -> Self::IntoIterator {
            self.files.clone()
        }
    }
}

#[cfg(test)]
mod test {
    use super::{Config, FsConfig};

    use std::fs::{self, File};
    use std::path::{Path, PathBuf};
    use std::io::prelude::*;

    use walkdir;

    fn clean_up(test_root: &PathBuf) {
        if test_root.exists() {
            fs::remove_dir_all(test_root).unwrap();
        }
        assert!(!test_root.is_dir());
    }

    fn set_up(suffix: &str, current: &str, shells: Vec<&str>) -> PathBuf {
        let test_root = PathBuf::from("./target/fs-config-tests-".to_owned() + suffix);

        clean_up(&test_root);
        fs::create_dir(&test_root).unwrap();
        assert!(test_root.is_dir());

        let path = test_root.join("current_shell");
        let mut file = File::create(&path).unwrap();
        file.write_all(current.as_bytes()).unwrap();

        let shell_root = test_root.join("shells");
        fs::create_dir(&shell_root).unwrap();
        for shell in shells {
            let new_shell = shell_root.join(PathBuf::from(shell));
            fs::create_dir(&new_shell).unwrap();
        }

        test_root
    }

    fn fs_config(test_root: impl AsRef<Path>) -> impl Config {
        let f = |opt_entry: Result<walkdir::DirEntry, walkdir::Error>| { opt_entry.ok() };
        let g = |entry: walkdir::DirEntry| { entry.path().to_owned() };
        FsConfig::new(&test_root, f, g)
    }
    #[test]
    fn has_a_root_path() {
        let test_root = set_up("root-path", "default", vec!["default"]);
        let config = fs_config(&test_root);
        assert_eq!(config.root_path(), &test_root);
    }

    #[test]
    fn returns_the_current_shell_name() {
        let test_root = set_up("current-shell-name", "current", vec!["current"]);
        let config = fs_config(&test_root);

        assert_eq!(*config.current_shell_name().unwrap(), "current".to_string());
    }

    #[test]
    fn can_set_the_current_shell_name() {
        let test_root = set_up("set-current-shell-name", "default", vec!["default"]);
        let mut config = fs_config(&test_root);
        config.set_current_shell_name("current").unwrap();

        let mut config_file = File::open(&test_root.join("current_shell")).unwrap();
        let mut name_on_disk = String::new();
        config_file.read_to_string(&mut name_on_disk).unwrap();

        let current = "current".to_string();
        assert_eq!(*config.current_shell_name().unwrap(), current);
        assert_eq!(name_on_disk, current);
    }

    #[test]
    fn can_confirm_a_shell_exists() {
        let test_root = set_up("confirm-shell-existence",
                               "default",
                               vec!["default", "other"]);
        let config = fs_config(&test_root);

        assert!(config.shell_exists("other"));
    }

    #[test]
    fn can_confirm_a_shell_does_not_exist() {
        let test_root = set_up("confirm-shell-non-existence",
                               "default",
                               vec!["default", "other"]);
        let config = fs_config(&test_root);

        assert!(!config.shell_exists("another"));
    }

    #[test]
    fn can_walk_a_directory() {
        let test_root = set_up("walk-directory",
                               "default",
                               vec!["default"]);
        let mut config = fs_config(&test_root);
        let shell_root = config.shell_root_path().join("default");
        fs::File::create(&shell_root.join("file1")).expect("Failed to create test file");

        let files = config.shell_files("default")
            .into_iter()
            .map(|f| f.to_string_lossy().to_string())
            .collect::<Vec<_>>();
        assert_eq!(files, vec!["file1"]);
    }
}

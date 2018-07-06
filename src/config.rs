use std::fs::File;
use std::{io, fs};
use std::io::prelude::*;
use std::path::{Path, PathBuf};

pub trait Config {
    fn initialize(&mut self) -> io::Result<()>;

    fn root_path(&self) -> &PathBuf;

    fn shell_root_path(&self) -> PathBuf;

    fn current_shell_name(&self) -> Option<String>;

    fn set_current_shell_name(&mut self, name: &str) -> io::Result<()>;

    fn does_shell_exist(&self, name: &str) -> bool;

    fn get_shell_list(&self) -> io::Result<Vec<String>>;
}

#[derive(Clone)]
pub struct FsConfig {
    pub root_path: PathBuf,
    pub current_shell: Option<String>,
}

impl FsConfig {
    pub fn new<P: AsRef<Path>>(root_path: P) -> Self {
        let root_path = PathBuf::from(root_path.as_ref());
        FsConfig {
            root_path: root_path,
            current_shell: None,
        }
    }

    fn read_current_shell(&self) -> io::Result<String> {
        let config_path = self.root_path.join("current_shell");

        let mut file = try!(File::open(&config_path));
        let mut current_shell = String::new();

        try!(file.read_to_string(&mut current_shell));

        Ok(current_shell)
    }

    fn config_path(&self) -> PathBuf {
        self.root_path.join("current_shell")
    }
}

impl Config for FsConfig {
    fn initialize(&mut self) -> io::Result<()> {
        let current_shell = try!(self.read_current_shell());
        self.current_shell = Some(current_shell);

        Ok(())
    }

    fn root_path(&self) -> &PathBuf {
        &self.root_path
    }

    fn shell_root_path(&self) -> PathBuf {
        self.root_path.join("shells")
    }

    fn current_shell_name(&self) -> Option<String> {
        self.current_shell.clone()
    }

    fn set_current_shell_name(&mut self, name: &str) -> io::Result<()> {
        let mut file = try!(File::create(&self.config_path()));

        try!(file.write_all(name.as_bytes()));

        self.current_shell = Some(name.to_string());

        Ok(())
    }

    fn does_shell_exist(&self, name: &str) -> bool {
        let shell_path = self.root_path.join("shells").join(name);
        shell_path.is_dir()
    }

    fn get_shell_list(&self) -> io::Result<Vec<String>> {
        let mut shell_names = Vec::new();
        let root_path = self.shell_root_path();
        if try!(fs::metadata(&root_path)).is_dir() {
            for entry in try!(fs::read_dir(&root_path)) {
                let entry = try!(entry);
                if try!(fs::metadata(entry.path())).is_dir() {
                    match entry.file_name().into_string() {
                        Ok(v) => shell_names.push(v),
                        Err(_err) => (),
                    }
                }
            }
        }
        shell_names.sort();
        return Ok(shell_names);
    }
}

#[cfg(test)]
pub mod mock {
    use std::io;
    use std::path::PathBuf;

    use super::Config;

    #[derive(Clone,Debug,Eq,PartialEq)]
    pub struct MockConfig {
        pub root_path: PathBuf,
        pub current_shell: String,
        pub allowed_shell_names: Vec<String>,
    }

    impl Config for MockConfig {
        fn initialize(&mut self) -> io::Result<()> {
            Ok(())
        }

        fn root_path(&self) -> &PathBuf {
            &self.root_path
        }

        fn shell_root_path(&self) -> PathBuf {
            self.root_path.join("shells")
        }

        fn current_shell_name(&self) -> Option<String> {
            Some(self.current_shell.clone())
        }

        fn set_current_shell_name(&mut self, name: &str) -> io::Result<()> {
            self.current_shell = name.to_owned();
            Ok(())
        }

        fn does_shell_exist(&self, name: &str) -> bool {
            self.allowed_shell_names.contains(&name.to_owned())
        }

        fn get_shell_list(&self) -> io::Result<Vec<String>> {
            let mut shell_names = self.allowed_shell_names.to_owned();
            shell_names.sort();
            return Ok(shell_names);
        }
    }
}

#[cfg(test)]
mod test {
    use std::fs;
    use std::fs::File;
    use std::path::PathBuf;
    use std::io::prelude::*;
    use super::{Config, FsConfig};
    use std::os::unix::ffi::OsStringExt;
    use std::ffi::OsString;

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

    #[test]
    fn has_a_root_path() {
        let test_root = set_up("root-path", "default", vec!["default"]);
        let config = FsConfig::new(test_root.clone());
        assert_eq!(config.root_path(), &test_root);

        clean_up(&test_root);
    }

    #[test]
    fn returns_the_current_shell_name() {
        let test_root = set_up("current-shell-name", "current", vec!["default"]);
        let mut config = FsConfig::new(test_root.clone());
        config.initialize().expect("Reading shell_name config file");

        assert_eq!(config.current_shell_name().unwrap(), "current".to_string());

        clean_up(&test_root);
    }

    #[test]
    fn can_set_the_current_shell_name() {
        let test_root = set_up("set-current-shell-name", "default", vec!["default"]);
        let mut config = FsConfig::new(test_root.clone());
        config.set_current_shell_name("current").unwrap();

        let mut config_file = File::open(&test_root.join("current_shell")).unwrap();
        let mut name_on_disk = String::new();
        config_file.read_to_string(&mut name_on_disk).unwrap();

        let current = "current".to_string();
        assert_eq!(config.current_shell_name().unwrap(), current);
        assert_eq!(name_on_disk, current);

        clean_up(&test_root);
    }

    #[test]
    fn can_confirm_a_shell_exists() {
        let test_root = set_up("confirm-shell-existence",
                               "default",
                               vec!["default", "other"]);
        let config = FsConfig::new(test_root.clone());

        assert!(config.does_shell_exist("other"));

        clean_up(&test_root);
    }

    #[test]
    fn can_confirm_a_shell_does_not_exist() {
        let test_root = set_up("confirm-shell-non-existence",
                               "default",
                               vec!["default", "other"]);
        let config = FsConfig::new(test_root.clone());

        assert!(!config.does_shell_exist("another"));

        clean_up(&test_root);
    }

    #[test]
    fn can_get_inhabitable_shells() {
        let test_root = set_up("can_get_inhabitable_shells",
                               "default",
                               vec!["default", "bcd", "abc", "cde"]);
        let config = FsConfig::new(test_root.clone());
        let res = config.get_shell_list().unwrap();
        
        assert_eq!(res, vec!["abc", "bcd", "cde", "default"]);
        clean_up(&test_root);
    }

    #[test]
    fn can_get_zero_inhabitable_shells() {
        let test_root = set_up("can_get_zero_inhabitable-shells", "", Vec::new());
        let config = FsConfig::new(test_root.clone());
        let res = config.get_shell_list();

        assert_eq!(res.unwrap().len(), 0);
        clean_up(&test_root);
    }

    #[test]
    fn cant_get_inhabitable_shells_for_nonexistant_shell_root() {
        let config = FsConfig::new(PathBuf::from("not_a_path"));
        let res = config.get_shell_list();
        assert!(res.is_err());
    }

    #[test]
    fn can_ignore_in_inhabitable_shells_non_unicode_char() {
        let test_root = set_up("can_ignore_bad_characters",
                               "default",
                               vec!["default", "bcd", "abc", "cde"]);
        let non_unicode = OsString::from_vec((vec![245, 246, 247, 245]));
        let shell_root = test_root.join("shells").join(non_unicode);
        fs::create_dir(&shell_root).unwrap();
        let config = FsConfig::new(test_root.clone());
        let res = config.get_shell_list().unwrap();
        assert_eq!(res, vec!["abc", "bcd","cde","default"]);
        clean_up(&test_root);
    }
}

pub use self::types::*;

use self::parse::*;
use clap::ArgMatches;
use coveralls_api::CiService;
use humantime_serde::deserialize as humantime_serde;
use log::{error, info, warn};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{Error, ErrorKind, Read};
use std::path::{Path, PathBuf};
use std::time::Duration;

mod parse;
pub mod types;

pub struct ConfigWrapper(pub Vec<Config>);

/// Specifies the current configuration tarpaulin is using.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub name: String,
    /// Path to the projects cargo manifest
    #[serde(rename = "manifest-path")]
    pub manifest: PathBuf,
    /// Path to a tarpaulin.toml config file
    pub config: Option<PathBuf>,
    /// Path to the projects cargo manifest
    pub root: Option<String>,
    /// Flag to also run tests with the ignored attribute
    #[serde(rename = "ignored")]
    pub run_ignored: bool,
    /// Flag to ignore test functions in coverage statistics
    #[serde(rename = "ignore-tests")]
    pub ignore_tests: bool,
    /// Ignore panic macros in code.
    #[serde(rename = "ignore-panics")]
    pub ignore_panics: bool,
    /// Flag to add a clean step when preparing the target project
    #[serde(rename = "force-clean")]
    pub force_clean: bool,
    /// Verbose flag for printing information to the user
    pub verbose: bool,
    /// Debug flag for printing internal debugging information to the user
    pub debug: bool,
    /// Flag to count hits in coverage
    pub count: bool,
    /// Flag specifying to run line coverage (default)
    #[serde(rename = "line")]
    pub line_coverage: bool,
    /// Flag specifying to run branch coverage
    #[serde(rename = "branch")]
    pub branch_coverage: bool,
    /// Directory to write output files
    #[serde(rename = "output-dir")]
    pub output_directory: PathBuf,
    /// Key relating to coveralls service or repo
    pub coveralls: Option<String>,
    /// Enum representing CI tool used.
    #[serde(rename = "ciserver", deserialize_with = "deserialize_ci_server")]
    pub ci_tool: Option<CiService>,
    /// Only valid if coveralls option is set. If coveralls option is set,
    /// as well as report_uri, then the report will be sent to this endpoint
    /// instead.
    #[serde(rename = "report-uri")]
    pub report_uri: Option<String>,
    /// Forward unexpected signals back to the tracee. Used for tests which
    /// rely on signals to work.
    #[serde(rename = "forward")]
    pub forward_signals: bool,
    /// Include all available features in target build
    #[serde(rename = "all-features")]
    pub all_features: bool,
    /// Do not include default features in target build
    #[serde(rename = "no-default-features")]
    pub no_default_features: bool,
    /// Build all packages in the workspace
    #[serde(alias = "workspace")]
    pub all: bool,
    /// Duration to wait before a timeout occurs
    #[serde(deserialize_with = "humantime_serde", rename = "timeout")]
    pub test_timeout: Duration,
    /// Build in release mode
    pub release: bool,
    /// Build the tests only don't run coverage
    #[serde(rename = "no-run")]
    pub no_run: bool,
    /// Don't update `Cargo.lock`.
    pub locked: bool,
    /// Don't update `Cargo.lock` or any caches.
    pub frozen: bool,
    /// Directory for generated artifacts
    #[serde(rename = "target-dir")]
    pub target_dir: Option<PathBuf>,
    /// Run tarpaulin on project without accessing the network
    pub offline: bool,
    /// Types of tests for tarpaulin to collect coverage on
    #[serde(rename = "run-types")]
    pub run_types: Vec<RunType>,
    /// Packages to include when building the target project
    pub packages: Vec<String>,
    /// Packages to exclude from testing
    pub exclude: Vec<String>,
    /// Files to exclude from testing in their compiled form
    #[serde(skip_deserializing, skip_serializing)]
    excluded_files: RefCell<Vec<Regex>>,
    /// Files to exclude from testing in uncompiled form (for serde)
    #[serde(rename = "exclude-files")]
    excluded_files_raw: Vec<String>,
    /// Varargs to be forwarded to the test executables.
    #[serde(rename = "args")]
    pub varargs: Vec<String>,
    /// Features to include in the target project build
    pub features: Vec<String>,
    /// Unstable cargo features to use
    #[serde(rename = "Z")]
    pub unstable_features: Vec<String>,
    /// Output files to generate
    #[serde(rename = "out")]
    pub generate: Vec<OutputFile>,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            name: String::new(),
            run_types: vec![RunType::Tests],
            manifest: default_manifest(),
            config: None,
            root: Default::default(),
            run_ignored: false,
            ignore_tests: false,
            ignore_panics: false,
            force_clean: false,
            verbose: false,
            debug: false,
            count: false,
            line_coverage: true,
            branch_coverage: false,
            generate: vec![],
            output_directory: Default::default(),
            coveralls: None,
            ci_tool: None,
            report_uri: None,
            forward_signals: false,
            no_default_features: false,
            features: vec![],
            unstable_features: vec![],
            all: false,
            packages: vec![],
            exclude: vec![],
            excluded_files: RefCell::new(vec![]),
            excluded_files_raw: vec![],
            varargs: vec![],
            test_timeout: Duration::from_secs(60),
            release: false,
            all_features: false,
            no_run: false,
            locked: false,
            frozen: false,
            target_dir: None,
            offline: false,
        }
    }
}

impl<'a> From<&'a ArgMatches<'a>> for ConfigWrapper {
    fn from(args: &'a ArgMatches<'a>) -> Self {
        info!("Creating config");
        let debug = args.is_present("debug");
        let verbose = args.is_present("verbose") || debug;
        let excluded_files = get_excluded(args);
        let excluded_files_raw = get_list(args, "exclude-files");

        let args_config = Config {
            name: String::new(),
            manifest: get_manifest(args),
            config: None,
            root: get_root(args),
            run_types: get_run_types(args),
            run_ignored: args.is_present("ignored"),
            ignore_tests: args.is_present("ignore-tests"),
            ignore_panics: args.is_present("ignore-panics"),
            force_clean: args.is_present("force-clean"),
            verbose,
            debug,
            count: args.is_present("count"),
            line_coverage: get_line_cov(args),
            branch_coverage: get_branch_cov(args),
            generate: get_outputs(args),
            output_directory: get_output_directory(args),
            coveralls: get_coveralls(args),
            ci_tool: get_ci(args),
            report_uri: get_report_uri(args),
            forward_signals: args.is_present("forward"),
            all_features: args.is_present("all-features"),
            no_default_features: args.is_present("no-default-features"),
            features: get_list(args, "features"),
            unstable_features: get_list(args, "Z"),
            all: args.is_present("all") | args.is_present("workspace"),
            packages: get_list(args, "packages"),
            exclude: get_list(args, "exclude"),
            excluded_files: RefCell::new(excluded_files.clone()),
            excluded_files_raw: excluded_files_raw.clone(),
            varargs: get_list(args, "args"),
            test_timeout: get_timeout(args),
            release: args.is_present("release"),
            no_run: args.is_present("no-run"),
            locked: args.is_present("locked"),
            frozen: args.is_present("frozen"),
            target_dir: get_target_dir(args),
            offline: args.is_present("offline"),
        };
        if args.is_present("ignore-config") {
            Self(vec![args_config])
        } else if args.is_present("config") {
            let mut path = PathBuf::from(args.value_of("config").unwrap());
            if path.is_relative() {
                path = env::current_dir()
                    .unwrap()
                    .join(path)
                    .canonicalize()
                    .unwrap();
            }
            let confs = Config::load_config_file(&path);
            Config::get_config_vec(confs, args_config)
        } else {
            if let Some(cfg) = args_config.check_for_configs() {
                let confs = Config::load_config_file(&cfg);
                Config::get_config_vec(confs, args_config)
            } else {
                Self(vec![args_config])
            }
        }
    }
}

impl Config {
    pub fn get_config_vec(file_configs: std::io::Result<Vec<Self>>, backup: Self) -> ConfigWrapper {
        if file_configs.is_err() {
            warn!("Failed to deserialize config file falling back to provided args");
            ConfigWrapper(vec![backup])
        } else {
            let mut confs = file_configs.unwrap();
            for c in confs.iter_mut() {
                c.merge(&backup);
            }
            if confs.is_empty() {
                ConfigWrapper(vec![backup])
            } else {
                ConfigWrapper(confs)
            }
        }
    }

    /// Taking an existing config look for any relevant config files
    pub fn check_for_configs(&self) -> Option<PathBuf> {
        if let Some(root) = &self.root {
            Self::check_path_for_configs(&root)
        } else {
            if let Some(root) = self.manifest.clone().parent() {
                Self::check_path_for_configs(&root)
            } else {
                None
            }
        }
    }

    fn check_path_for_configs<P: AsRef<Path>>(path: P) -> Option<PathBuf> {
        let mut path_1 = PathBuf::from(path.as_ref());
        let mut path_2 = path_1.clone();
        path_1.push("tarpaulin.toml");
        path_2.push(".tarpaulin.toml");
        if path_1.exists() {
            Some(path_1)
        } else if path_2.exists() {
            Some(path_2)
        } else {
            None
        }
    }

    pub fn load_config_file<P: AsRef<Path>>(file: P) -> std::io::Result<Vec<Self>> {
        let mut f = File::open(file.as_ref())?;
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer)?;
        let mut res = Self::parse_config_toml(&buffer);
        if let Ok(cfs) = res.as_mut() {
            for mut c in cfs.iter_mut() {
                c.config = Some(file.as_ref().to_path_buf());
            }
        }
        res
    }

    pub fn parse_config_toml(buffer: &[u8]) -> std::io::Result<Vec<Self>> {
        let mut map: HashMap<String, Self> = toml::from_slice(&buffer).map_err(|e| {
            error!("Invalid config file {}", e);
            Error::new(ErrorKind::InvalidData, format!("{}", e))
        })?;

        let mut result = Vec::new();
        for (name, mut conf) in map.iter_mut() {
            conf.name = name.to_string();
            result.push(conf.clone());
        }
        if result.is_empty() {
            Err(Error::new(ErrorKind::InvalidData, "No config tables"))
        } else {
            Ok(result)
        }
    }

    /// Given a config made from args ignoring the config file take the
    /// relevant settings that should be carried across and move them
    pub fn merge(&mut self, other: &Config) {
        if other.debug {
            self.debug = other.debug;
            self.verbose = other.verbose;
        } else if other.verbose {
            self.verbose = other.verbose;
        }
        self.manifest = other.manifest.clone();
        self.root = other.root.clone();
        if !other.excluded_files_raw.is_empty() {
            self.excluded_files_raw
                .extend_from_slice(&other.excluded_files_raw);

            // Now invalidated the compiled regex cache so clear it
            let mut excluded_files = self.excluded_files.borrow_mut();
            excluded_files.clear();
        }
    }

    #[inline]
    pub fn is_coveralls(&self) -> bool {
        self.coveralls.is_some()
    }

    #[inline]
    pub fn exclude_path(&self, path: &Path) -> bool {
        if self.excluded_files.borrow().len() != self.excluded_files_raw.len() {
            let mut excluded_files = self.excluded_files.borrow_mut();
            let mut compiled = regexes_from_excluded(&self.excluded_files_raw);
            excluded_files.clear();
            excluded_files.append(&mut compiled);
        }
        let project = self.strip_base_dir(path);

        self.excluded_files
            .borrow()
            .iter()
            .any(|x| x.is_match(project.to_str().unwrap_or("")))
    }

    ///
    /// returns the relative path from the base_dir
    /// uses root if set, else env::current_dir()
    ///
    #[inline]
    pub fn get_base_dir(&self) -> PathBuf {
        if let Some(root) = &self.root {
            if Path::new(root).is_absolute() {
                PathBuf::from(root)
            } else {
                let base_dir = env::current_dir().unwrap();
                base_dir.join(root).canonicalize().unwrap()
            }
        } else {
            env::current_dir().unwrap()
        }
    }

    /// returns the relative path from the base_dir
    ///
    #[inline]
    pub fn strip_base_dir(&self, path: &Path) -> PathBuf {
        path_relative_from(path, &self.get_base_dir()).unwrap_or_else(|| path.to_path_buf())
    }

    #[inline]
    pub fn is_default_output_dir(&self) -> bool {
        self.output_directory == env::current_dir().unwrap()
    }
}

/// Gets the relative path from one directory to another, if it exists.
/// Credit to brson from this commit from 2015
/// https://github.com/rust-lang/rust/pull/23283/files
///
fn path_relative_from(path: &Path, base: &Path) -> Option<PathBuf> {
    use std::path::Component;

    if path.is_absolute() != base.is_absolute() {
        if path.is_absolute() {
            Some(path.to_path_buf())
        } else {
            None
        }
    } else {
        let mut ita = path.components();
        let mut itb = base.components();
        let mut comps = vec![];

        loop {
            match (ita.next(), itb.next()) {
                (None, None) => break,
                (Some(a), None) => {
                    comps.push(a);
                    comps.extend(ita.by_ref());
                    break;
                }
                (None, _) => comps.push(Component::ParentDir),
                (Some(a), Some(b)) if comps.is_empty() && a == b => (),
                (Some(a), Some(b)) if b == Component::CurDir => comps.push(a),
                (Some(_), Some(b)) if b == Component::ParentDir => return None,
                (Some(a), Some(_)) => {
                    comps.push(Component::ParentDir);
                    for _ in itb {
                        comps.push(Component::ParentDir);
                    }
                    comps.push(a);
                    comps.extend(ita.by_ref());
                    break;
                }
            }
        }
        Some(comps.iter().map(|c| c.as_os_str()).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::App;

    #[test]
    fn exclude_paths() {
        let matches = App::new("tarpaulin")
            .args_from_usage("--exclude-files [FILE]... 'Exclude given files from coverage results has * wildcard'")
            .get_matches_from_safe(vec!["tarpaulin", "--exclude-files", "*module*"])
            .unwrap();
        let conf = ConfigWrapper::from(&matches).0;
        assert_eq!(conf.len(), 1);
        assert!(conf[0].exclude_path(Path::new("src/module/file.rs")));
        assert!(!conf[0].exclude_path(Path::new("src/mod.rs")));
        assert!(!conf[0].exclude_path(Path::new("unrelated.rs")));
        assert!(conf[0].exclude_path(Path::new("module.rs")));
    }

    #[test]
    fn no_exclusions() {
        let matches = App::new("tarpaulin")
            .args_from_usage("--exclude-files [FILE]... 'Exclude given files from coverage results has * wildcard'")
            .get_matches_from_safe(vec!["tarpaulin"])
            .unwrap();
        let conf = ConfigWrapper::from(&matches).0;
        assert_eq!(conf.len(), 1);
        assert!(!conf[0].exclude_path(Path::new("src/module/file.rs")));
        assert!(!conf[0].exclude_path(Path::new("src/mod.rs")));
        assert!(!conf[0].exclude_path(Path::new("unrelated.rs")));
        assert!(!conf[0].exclude_path(Path::new("module.rs")));
    }

    #[test]
    fn exclude_exact_file() {
        let matches = App::new("tarpaulin")
            .args_from_usage("--exclude-files [FILE]... 'Exclude given files from coverage results has * wildcard'")
            .get_matches_from_safe(vec!["tarpaulin", "--exclude-files", "*/lib.rs"])
            .unwrap();
        let conf = ConfigWrapper::from(&matches).0;
        assert_eq!(conf.len(), 1);
        assert!(conf[0].exclude_path(Path::new("src/lib.rs")));
        assert!(!conf[0].exclude_path(Path::new("src/mod.rs")));
        assert!(!conf[0].exclude_path(Path::new("src/notlib.rs")));
        assert!(!conf[0].exclude_path(Path::new("lib.rs")));
    }

    #[test]
    fn relative_path_test() {
        let path_a = Path::new("/this/should/form/a/rel/path/");
        let path_b = Path::new("/this/should/form/b/rel/path/");

        let rel_path = path_relative_from(path_b, path_a);
        assert!(rel_path.is_some());
        assert_eq!(
            rel_path.unwrap().to_str().unwrap(),
            "../../../b/rel/path",
            "Wrong relative path"
        );

        let path_a = Path::new("/this/should/not/form/a/rel/path/");
        let path_b = Path::new("./this/should/not/form/a/rel/path/");

        let rel_path = path_relative_from(path_b, path_a);
        assert_eq!(rel_path, None, "Did not expect relative path");

        let path_a = Path::new("./this/should/form/a/rel/path/");
        let path_b = Path::new("./this/should/form/b/rel/path/");

        let rel_path = path_relative_from(path_b, path_a);
        assert!(rel_path.is_some());
        assert_eq!(
            rel_path.unwrap().to_str().unwrap(),
            "../../../b/rel/path",
            "Wrong relative path"
        );
    }

    #[test]
    fn config_toml() {
        let toml = "[global]
        ignored= true
        coveralls= \"hello\"

        [other]
        run-types = [\"Doctests\", \"Tests\"]";

        let configs = Config::parse_config_toml(toml.as_bytes()).unwrap();
        assert_eq!(configs.len(), 2);
        for c in &configs {
            if c.name == "global" {
                assert_eq!(c.run_ignored, true);
                assert_eq!(c.coveralls, Some("hello".to_string()));
            } else if c.name == "other" {
                assert_eq!(c.run_types, vec![RunType::Doctests, RunType::Tests]);
            } else {
                panic!("Unexpected name {}", c.name);
            }
        }
    }

    #[test]
    fn excluded_merge() {
        let toml = r#"[a]
        exclude-files = ["target/*"]
        [b]
        exclude-files = ["foo.rs"]
        "#;

        let mut configs = Config::parse_config_toml(toml.as_bytes()).unwrap();
        let mut config = configs.remove(0);
        config.merge(&configs[0]);
        println!("{:?}", configs[0].excluded_files_raw);
        println!("{:?}", config.excluded_files_raw);
        assert!(config.excluded_files_raw.contains(&"target/*".to_string()));
        assert!(config.excluded_files_raw.contains(&"foo.rs".to_string()));

        assert_eq!(config.excluded_files_raw.len(), 2);
        assert_eq!(configs[0].excluded_files_raw.len(), 1);
    }

    #[test]
    fn all_toml_options() {
        let toml = r#"[all]
        debug = true
        verbose = true
        ignore-panics = true
        count = true
        ignored = true
        force-clean = true
        branch = true
        forward = true
        coveralls = "hello"
        report-uri = "http://hello.com"
        no-default-features = true
        features = ["a"]
        all-features = true
        workspace = true
        packages = ["pack_1"]
        exclude = ["pack_2"]
        exclude-files = ["fuzz/*"]
        timeout = "5s"
        release = true
        no-run = true
        locked = true
        frozen = true
        target-dir = "/tmp"
        offline = true
        Z = ["something-nightly"]
        out = ["Html"]
        run-types = ["Doctests"]
        root = "/home/rust"
        manifest-path = "/home/rust/foo/Cargo.toml"
        ciserver = "travis-ci"
        args = ["--nocapture"]
        "#;
        let mut configs = Config::parse_config_toml(toml.as_bytes()).unwrap();
        assert_eq!(configs.len(), 1);
        let config = configs.remove(0);
        assert!(config.debug);
        assert!(config.verbose);
        assert!(config.ignore_panics);
        assert!(config.count);
        assert!(config.run_ignored);
        assert!(config.force_clean);
        assert!(config.branch_coverage);
        assert!(config.forward_signals);
        assert_eq!(config.coveralls, Some("hello".to_string()));
        assert_eq!(config.report_uri, Some("http://hello.com".to_string()));
        assert!(config.no_default_features);
        assert!(config.all_features);
        assert!(config.all);
        assert!(config.release);
        assert!(config.no_run);
        assert!(config.locked);
        assert!(config.frozen);
        assert!(config.offline);
        assert_eq!(config.test_timeout, Duration::from_secs(5));
        assert_eq!(config.unstable_features.len(), 1);
        assert_eq!(config.unstable_features[0], "something-nightly");
        assert_eq!(config.varargs.len(), 1);
        assert_eq!(config.varargs[0], "--nocapture");
        assert_eq!(config.features.len(), 1);
        assert_eq!(config.features[0], "a");
        assert_eq!(config.excluded_files_raw.len(), 1);
        assert_eq!(config.excluded_files_raw[0], "fuzz/*");
        assert_eq!(config.packages.len(), 1);
        assert_eq!(config.packages[0], "pack_1");
        assert_eq!(config.exclude.len(), 1);
        assert_eq!(config.exclude[0], "pack_2");
        assert_eq!(config.generate.len(), 1);
        assert_eq!(config.generate[0], OutputFile::Html);
        assert_eq!(config.run_types.len(), 1);
        assert_eq!(config.run_types[0], RunType::Doctests);
        assert_eq!(config.ci_tool, Some(CiService::Travis));
        assert_eq!(config.root, Some("/home/rust".to_string()));
        assert_eq!(config.manifest, PathBuf::from("/home/rust/foo/Cargo.toml"));
    }
}

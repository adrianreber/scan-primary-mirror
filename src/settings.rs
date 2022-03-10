use config::{Config, ConfigError, Environment, File};
use serde_derive::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Database {
    pub url: String,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct RepositoryAlias {
    /// There are a small number of repositories which do not follow
    /// the expected naming scheme. This offers the possibility
    /// to create an alias for certain repositories:
    ///
    /// # Examples
    ///
    /// ```
    /// [[repository_alias]]
    /// from="testing-modular-epel-debug-"
    /// to="testing-modular-debug-epel"
    /// ```
    pub from: String,
    pub to: String,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct RepositoryMapping {
    /// Each `regex` is used to create from the directory a repository `prefix`
    ///
    /// # Examples
    ///
    /// ```
    /// [[repository_mapping]]
    /// regex="^free/fedora/updates/[\\.\\d]+/.*"
    /// prefix="free-fedora-updates-released"
    /// ```
    pub regex: String,
    /// This is the corresponding prefix which should be used for the
    /// above mentioned `regex`.
    ///
    /// The prefix will be followed by the version and optional by
    /// `-debug` or `-source`.
    pub prefix: String,
    /// version_prefix: optional prefix for version in the resulting
    /// repository prefix. f35 instead of 35
    pub version_prefix: Option<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct Category {
    /// name: category name like in the MM database
    pub name: String,
    /// type: rsync or directory
    pub r#type: String,
    /// url: rsync url or path
    pub url: String,
    /// options: additional rsync parameters
    pub options: Option<String>,
    /// checksum_base: in case of rsync this needs to be an http(s)
    /// url used to download repomd.xml for hashsum generation
    pub checksum_base: Option<String>,
    /// excludes: comma separated list of regex for directories to exclude
    pub excludes: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    /// rsync options added for all categories
    pub common_rsync_options: Option<String>,
    pub max_propagation_days: Option<i64>,
    pub max_stale_days: Option<i64>,
    pub debug: Option<bool>,
    pub database: Database,
    /// Comma separated list of regex for directories to exclude.
    /// This will be combined with the category specific excludes.
    pub excludes: Option<Vec<String>>,
    /// Comma separated list of path prefixes which should be ignored
    /// when guessing the version from a given path.
    pub skip_paths_for_version: Option<Vec<String>>,
    /// Comma separated list of path elements which would make a version
    /// a test release
    pub test_paths: Option<Vec<String>>,
    /// Comma separated list of path elements which should skip
    /// repository creation.
    pub skip_repository_paths: Option<Vec<String>>,
    /// Comma separated list of path elements which should not be
    /// shown in the mirror list matrix
    pub do_not_display_paths: Option<Vec<String>>,
    /// At least one category is required
    pub category: Option<Vec<Category>>,
    /// Used to create repository prefixes
    pub repository_mapping: Option<Vec<RepositoryMapping>>,
    /// Some repository prefix cannot combined as
    /// expected by scan-primary-mirror and therefore
    /// repository aliases are needed.
    pub repository_aliases: Option<Vec<RepositoryAlias>>,
}

impl Settings {
    pub fn new(config_file: String) -> Result<Self, ConfigError> {
        let s = Config::builder()
            .add_source(File::with_name(&config_file))
            .add_source(Environment::with_prefix("UMDL").separator("__"))
            .build()?;
        s.try_deserialize()
    }
}

use std::{env, fs};

use crate::{GitlabConfig, ConfigToml};

pub fn get_gitlab_config () -> GitlabConfig {
    let path = env::current_dir().unwrap();
    let content = fs::read_to_string(path.join(".conf.toml")).unwrap();
    let setting: ConfigToml  = toml::from_str(&content).unwrap();
    return setting.gitlab;
}

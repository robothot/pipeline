

use std::{env, fs};

use log::info;

use crate::{GitlabConfig, ConfigToml};

pub fn get_gitlab_config () -> GitlabConfig {
    let path = env::current_dir().unwrap();
    let content = fs::read_to_string(path.join(".conf.toml")).unwrap();
    let setting: ConfigToml  = toml::from_str(&content).unwrap();
    return setting.gitlab;
}

pub async fn accept(project: u64, merge_request: u64) -> Result<reqwest::Response, reqwest::Error> {
    let gitlab = get_gitlab_config();
    let url = format!("{}://{}/api/v4/projects/{}/merge_requests/{}/merge", gitlab.protocol, gitlab.host, project, merge_request);
    info!("request -> {}", url);
    reqwest::Client::new()
    .put(url)
    .header("Private-Token", gitlab.token.as_str())
    .send()
    .await
}

pub async fn create(
    id: String,
    source_branch: String,
    target_branch: String,
    title: String
) -> Result<reqwest::Response, reqwest::Error> {
    info!("request -> create");
    let gitlab = get_gitlab_config();
    info!("request -> gitlab");
    let url = format!("{}://{}/api/v4/projects/{}/merge_requests", gitlab.protocol, gitlab.host, id);
    info!("request -> {}", url);
    reqwest::Client::new()
        .post(url)
        .header("Private-Token", gitlab.token.as_str())
        .header("Accept", "application/json; charset=utf-8")
        .body(format!(r#"{{ "title": "{}", "source_branch": "{}", "target_branch": "{}" }}"#, title, source_branch, target_branch))
        .send()
        .await
    }
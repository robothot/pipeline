use std::{env, fs, collections::HashMap};

use actix::Actor;
use actix_web::{ App, HttpServer, web::{ Data } };
use serde::Deserialize;

mod services;
mod actors;

#[derive(Deserialize, Clone)]
struct GitlabProject {
    // 分支名称
    branch: String,
    /**
     * -. 快速合并 quick://[merge type]
     * -. 定时合并  cron://[merge type]/[cron expression]
     */
    merge_request: Vec<String>,
}

#[derive(Deserialize, Clone)]
struct GitlabConfig {
    // 协议
    protocol: String,
    // 地址
    host: String,
    // 令牌
    token: String,
    // 项目
    project: HashMap<String, GitlabProject>
}

#[derive(Deserialize, Clone)]
pub struct ConfigToml {
    host: String,
    port: u16,
    gitlab: GitlabConfig
}


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();

    let path = env::current_dir()?;
    let content = fs::read_to_string(path.join(".conf.toml"))?;
    let setting: ConfigToml  = toml::from_str(&content).unwrap();

    let host = setting.host.clone();
    let port = setting.port;
  
    let addr = actors::pipeline::PipelineActor::new().start();
    let gitlab = actors::gitlab::GitLabActor::new(setting.clone()).start();
    
    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(setting.clone()))
            .app_data(Data::new(addr.clone()))
            .app_data(Data::new(gitlab.clone()))
            .service(services::web_hooks::webhook_events)
    })
    .bind((host.as_str(), port))?
    .run()
    .await
}




use std::{env, fs, collections::HashMap, str::FromStr, time::Duration};
use actix::Actor;
use actix_web::{ App, HttpServer, web::{ Data } };
use chrono::{FixedOffset, Local};
use cron::Schedule;
use log::info;
use serde::Deserialize;

mod services;
mod actors;
mod gitlab;

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
pub struct GitlabConfig {
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
    info!("version       : 0.0.15");
    info!("release time  : 2023-04-6 10:29");
    let path = env::current_dir()?;

    let content = fs::read_to_string(path.join(".conf.toml"))?;
    let setting: ConfigToml  = toml::from_str(&content).unwrap();
    let host = setting.host.clone();
    let port = setting.port;
  
    let addr = actors::pipeline::PipelineActor::new().start();

    let rt_setting = setting.clone();
    let project = rt_setting.gitlab.project;

    actix_rt::spawn(async move {
        let offset = Some(FixedOffset::east_opt(8 * 3600)).unwrap();
        let mut request: Vec<Vec<String>> = vec![];
        project.iter().for_each(|(project_id, item)| {
            item.merge_request.iter().for_each(|element| {
                if element.contains("cron://") {
                    let split_element: Vec<&str> = element.split("://").collect();
                    let params: Vec<&str> = split_element[1].split("/").collect();
                    let target_branch = params[0].to_string().clone();
                    let source_branch = item.branch.clone();
                    let expression = params[1].to_string().clone();
                    let project_id = project_id.clone();
                    request.push(vec![project_id, source_branch.to_string(), target_branch, expression]);
                }
            });
        });
        if &request.len() > &0 {
            loop {
                for element in &request {
                    let schedule = Schedule::from_str(element[3].as_str()).unwrap();
                    let mut upcoming = schedule.upcoming(offset.unwrap()).take(1);
                    actix_rt::time::sleep(Duration::from_millis(500)).await;
                    let local = &Local::now();
                    if let Some(datetime) = upcoming.next() {
                        if datetime.timestamp() <= local.timestamp() {
                            let _ = gitlab::mr::create(
                                element[0].clone().as_str(),
                                element[1].clone().as_str(),
                                element[2].clone().as_str(),
                                "[RUST-ROBOT]: 自动发版"
                            ).await;
                        }
                    }
                }
            }
        }
    });
   
    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(addr.clone()))
            .service(services::web_hooks::webhook_events)
    })
    .bind((host.as_str(), port))?
    .run()
    .await
}




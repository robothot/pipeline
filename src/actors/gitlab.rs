use actix::{Actor, Context, Handler, Message};
use gitlab::{Gitlab, api::{projects::merge_requests::{ MergeMergeRequest, RebaseMergeRequest }, Query}};

use crate::ConfigToml;


#[derive(Message)]
#[rtype(result = "()")]
pub struct MergeRequest {
    pub project: u64,
    pub merge_request: u64,
    pub target_branch: String
}

pub struct GitLabActor {
    gitlab: Gitlab,
    config: ConfigToml
}


impl GitLabActor {
    pub fn new(gitlab: Gitlab, config: ConfigToml) -> GitLabActor {
        GitLabActor {
            gitlab,
            config
        }
    }
} 

impl Actor for GitLabActor {
    type Context = Context<Self>;
}


impl Handler<MergeRequest> for GitLabActor{ 
    type Result = ();
    fn handle(&mut self, msg: MergeRequest, _: &mut Self::Context) {
        if let Some((_, project)) = self.config.gitlab.project.iter().find(|(_, item)| {
            item.branch.eq(&msg.target_branch)
        }) {
            if let Some(element) = project.merge_request.iter().find(|data| {
                let data: Vec<&str> = data.split("://").collect();
                data[0].eq("quick")
            }) {
                let data: Vec<&str> = element.split("://").collect();
                if "merge".eq(data[1]) {
                    let endpoint = MergeMergeRequest::builder()
                    .project(msg.project)
                    .merge_request(msg.merge_request)
                    .build()
                    .unwrap();
                    gitlab::api::ignore(endpoint).query(&self.gitlab).unwrap();
                } else if "rebase".eq(data[1]) {
                    let endpoint = RebaseMergeRequest::builder()
                    .project(msg.project)
                    .merge_request(msg.merge_request)
                    .build()
                    .unwrap();
                    gitlab::api::ignore(endpoint).query(&self.gitlab).unwrap();
                }
            }
        }

    }
}
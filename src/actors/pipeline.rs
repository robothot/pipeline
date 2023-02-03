use std::{error::Error, env, fs::{self}, process::{Command, Stdio}, sync::{mpsc::{self, Sender, Receiver}, Mutex}, io::BufRead, cell::RefCell};

use actix::{Actor, Context, Handler, Message };
use actix_web::web::{self, Data};
use log::{info, error};
use serde::Deserialize;
use serde_json::Value;


#[derive(Deserialize)]
struct PipelineTomlBuild {
    command: String,
    output_directory: String
}

#[derive(Deserialize)]
struct PipelineTomlPublishGit {
    // 仓库名称
    name: String,
    // 分支信息
    branch: String,
    // 仓库地址
    repository: String,
}

#[derive(Deserialize)]
struct PipelineTomlPublish {
    git: Vec<PipelineTomlPublishGit>
}

#[derive(Deserialize)]
struct PipelineToml {
    build: PipelineTomlBuild,
    publish: PipelineTomlPublish
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct RunPipeline {
    pub param: Value
}

pub struct PipelineActor {
    sender: Data<Sender<i64>>,
    receiver: Data<Mutex<Receiver<i64>>>,
    run_task: Data<Mutex<RefCell<Vec<i64>>>>,
}

impl PipelineActor {
    pub fn new() -> PipelineActor {
        let (tx, rx) = mpsc::channel::<i64>();
        
        PipelineActor {
            sender: Data::new(tx),
            receiver: Data::new(Mutex::new(rx)),
            run_task: Data::new(Mutex::new(RefCell::new(vec![])))
        }
    }

    fn run_build_script(param: Value, receiver: Data<Mutex<Receiver<i64>>>)-> Result<(), Box<dyn Error>>  {
        Self::run_pipeline_clone(&param)?;

        let path = env::current_dir()?;
        let project = param.get("project");
        let project_namespace = project.unwrap().get("namespace").unwrap().as_str().unwrap().to_string();
        let project_name = project.unwrap().get("name").unwrap().as_str().unwrap().to_string();
        let object_attributes = param.get("object_attributes");
        let id = object_attributes.unwrap().get("id").unwrap().as_i64().unwrap();
        let target_path = &path.join(&project_namespace).join(&project_name);
        let content = fs::read_to_string(target_path.join(".pipeline.toml"))?;
        let setting: PipelineToml  = toml::from_str(&content)?;
        let command = setting.build.command;

        info!("run command -> {} ", &command);
        let mut child = Command::new("sh")
            .current_dir(&target_path)
            .arg("-c")
            .arg(command)
            .stdout(Stdio::piped())
            .spawn()?;



        let out = child.stdout.take().unwrap();
        
        let mut out = std::io::BufReader::new(out);
        let mut buffer = String::new();
        while let Ok(_) = out.read_line(&mut buffer) {
            if let Ok(Some(_)) = child.try_wait() {
                break;
            }
           
            let receiver = receiver.try_lock().unwrap();
            if let Ok(msg) = receiver.try_recv() {
                if msg.eq(&id) {
                    info!("task kill {}", &id);
                    child.kill()?;
                    break;
                }
            }
            if buffer.is_empty() == false {
                info!("[{}] -> {}", &child.id(), buffer);
                buffer.clear();
            }
        }
        Ok(())
    }

    fn run_pipeline_clone(param: &Value) -> Result<(), Box<dyn Error>> {
        let path = env::current_dir()?;
        info!("current_dir -> {}", &path.display());
        let project = param.get("project");
        let project_namespace = project.unwrap().get("namespace").unwrap().as_str().unwrap().to_string();
        let project_name = project.unwrap().get("name").unwrap().as_str().unwrap().to_string();
        let object_attributes = param.get("object_attributes");
    
        let target = object_attributes.unwrap().get("target").unwrap();
        let target_http_url = target.get("git_http_url").clone().unwrap().as_str().unwrap().to_string();
        if path.join(&project_namespace).join(&project_name).exists() == false {
            if path.join(&project_namespace).exists() == false {
                fs::create_dir_all(path.join(&project_namespace).join(&project_name))?;
            }
            let target_path = &path.join(&project_namespace);
            let child = Command::new("sh")
            .current_dir(&target_path)
            .arg("-c")
            .arg(format!("git clone --depth 1 {} {}", &target_http_url, &project_namespace))
            .stdout(Stdio::piped())
            .spawn()?;
            child.wait_with_output()?;
        } else {
            let target_path = &path.join(&project_namespace).join(&project_name);
            let child = Command::new("sh")
            .current_dir(&target_path)
            .arg("-c")
            .arg(format!("git pull "))
            .stdout(Stdio::piped())
            .spawn()?;
            child.wait_with_output()?;
        }
        Ok(())
    }
}

impl Actor for PipelineActor {
    type Context = Context<Self>;
}

impl Handler<RunPipeline> for PipelineActor{
    type Result = ();
    fn handle(&mut self, msg: RunPipeline, _: &mut Self::Context) {
        let receiver =  self.receiver.clone();
        let sender = self.sender.clone();
        let param = msg.param.clone();
        let object_attributes = param.get("object_attributes");
        let id = object_attributes.unwrap().get("id").unwrap().as_i64().unwrap();
        
        let run_task = self.run_task.clone();
        let run_task_lock = run_task.clone();
        let run_task_lock = run_task_lock.try_lock().unwrap();
        let mut run_task_lock = run_task_lock.borrow_mut();
        if run_task_lock.contains(&id) {
            sender.send(id).unwrap();
        }
        run_task_lock.push(id);

        let _ = web::block(move || {

            let path = env::current_dir().unwrap();
            let project = param.get("project");
            let project_namespace = project.unwrap().get("namespace").unwrap().as_str().unwrap().to_string();
            let project_name = project.unwrap().get("name").unwrap().as_str().unwrap().to_string();
            let target_path = &path.join(&project_namespace).join(&project_name);
    

            match Self::run_build_script(msg.param.clone(), receiver.clone()) {
                Ok(()) => {

                    let content = fs::read_to_string(target_path.join(".pipeline.toml")).unwrap();
                    let setting: PipelineToml  = toml::from_str(&content).unwrap();
                    
                    for element in setting.publish.git {
                        let name = element.name;
                        let branch = element.branch;
                        let repository = element.repository;

                        let child = Command::new("sh")
                        .current_dir(&target_path.join(&setting.build.output_directory))
                        .arg("-c")
                        .arg(format!(
                            "git init && git checkout {} && git remote add {} {} && git push --force {} {} ",
                            &branch,
                            &name,
                            &repository,
                            &name,
                            &branch,
                        ))
                        .stdout(Stdio::piped())
                        .spawn().unwrap();
                        child.wait_with_output().unwrap();
                    }
                    

                },
                Err(err) => {
                    error!("ERR -> {}", err.to_string());
                }
            }
            let run_task = run_task.try_lock().unwrap();
            let binding = run_task.clone();
            let mut run_task = binding.borrow_mut();
            let position = run_task.iter().position(|item| item.eq(&id));
            if let Some(position) = position {
                run_task.remove(position);
            }
        });
    
    }
}


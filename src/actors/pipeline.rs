use std::{error::Error, env, fs::{self}, process::{Command, Stdio}, sync::{mpsc::{self, Sender, Receiver}, Mutex}, io::BufRead, cell::RefCell, collections::HashMap, fmt};

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

    // 设置环境变量指定 key GIT_SSH_COMMAND='ssh -i  ~/.ssh'
    ssh_key: Option<String>,

    // 分支信息
    branch: String,

    // 仓库地址
    repository: String,
}

#[derive(Deserialize)]
struct PipelineTomlPublish {
    git: HashMap<String, PipelineTomlPublishGit>
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

#[derive(Debug)]
struct PipelineError(String);


impl fmt::Display for PipelineError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for PipelineError {}

pub struct PipelineActor {
    sender: Data<Sender<String>>,
    receiver: Data<Mutex<Receiver<String>>>,
    run_task: Data<Mutex<RefCell<Vec<String>>>>,
}

impl PipelineActor {
    pub fn new() -> PipelineActor {
        let (tx, rx) = mpsc::channel::<String>();
        
        PipelineActor {
            sender: Data::new(tx),
            receiver: Data::new(Mutex::new(rx)),
            run_task: Data::new(Mutex::new(RefCell::new(vec![])))
        }
    }

    fn run_build_script(param: Value, receiver: Data<Mutex<Receiver<String>>>)-> Result<(), Box<dyn Error>>  {
        Self::run_pipeline_clone(&param)?;

        let path = env::current_dir()?.join(".cache");
        let project = param.get("project");
        let project_namespace = project.unwrap().get("namespace").unwrap().as_str().unwrap().to_string();
        let project_name = project.unwrap().get("name").unwrap().as_str().unwrap().to_string();
        let object_attributes = param.get("object_attributes");
        let id = object_attributes.unwrap().get("id").unwrap().as_i64().unwrap();
        let target_branch = object_attributes.unwrap().get("target_branch").unwrap().as_str().unwrap();
      
        let target_path = &path.join(&project_namespace).join(&project_name).join(&target_branch);
        let content = fs::read_to_string(target_path.join(".pipeline.toml"))?;
        let setting: PipelineToml  = toml::from_str(&content)?;
        let command = setting.build.command;

        let target_branch_id = object_attributes.unwrap().get("target_branch_id").unwrap().as_u64().unwrap();
        let project_id = project.unwrap().get("id").unwrap().as_u64().unwrap();

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
       
            if buffer.is_empty() == false {
                info!("[{}] -> {}", &child.id(), buffer);
                buffer.clear();
            }
            if let Ok(msg) = receiver.try_recv() {
                let ids = format!("{}/{}", project_id, target_branch_id);
                if msg.eq(&ids) {
                    info!("task kill {}", &id);
                    child.kill()?;
                    return Err(Box::new(PipelineError("kill".into())));
                }
            }
        }
        Ok(())
    }

    fn run_pipeline_clone(param: &Value) -> Result<(), Box<dyn Error>> {
        let path =env::current_dir()?.join(".cache");
        let project = param.get("project");
        let project_namespace = project.unwrap().get("namespace").unwrap().as_str().unwrap().to_string();
        let project_name = project.unwrap().get("name").unwrap().as_str().unwrap().to_string();
        let object_attributes = param.get("object_attributes");
    
        let target = object_attributes.unwrap().get("target").unwrap();
        let target_branch = object_attributes.unwrap().get("target_branch").unwrap().as_str().unwrap();
        let target_http_url = target.get("git_http_url").clone().unwrap().as_str().unwrap().to_string();
        let dir = path.join(&project_namespace).join(&project_name);
        if dir.join(target_branch).exists() == false {
            fs::create_dir_all(&dir)?;
            let child = Command::new("sh")
            .current_dir(&dir)
            .arg("-c")
            .arg(format!("git clone -b {} --depth 1 {} {}", &target_branch, &target_http_url, &target_branch))
            .stdout(Stdio::piped())
            .spawn()?;
            child.wait_with_output()?;
        } else {
            let child = Command::new("sh")
            .current_dir(&dir.join(target_branch))
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
        let target_branch = object_attributes.unwrap().get("target_branch").unwrap().as_str().unwrap().to_string();
        let run_task = self.run_task.clone();
        let target_branch_id = object_attributes.unwrap().get("target_branch_id").unwrap().as_u64().unwrap();
        let project = param.get("project");
        let project_id = project.unwrap().get("id").unwrap().as_u64().unwrap();
        let ids = format!("{}/{}", project_id, target_branch_id);
        {
            let run_task_lock = run_task.clone();
            let run_task_lock = run_task_lock.lock().unwrap();
            let mut run_task_lock = run_task_lock.borrow_mut();

            if run_task_lock.contains(&ids) {
                sender.send(ids.clone()).unwrap();
            }
            run_task_lock.push(ids.clone());
        }

        let _ = web::block(move || {

            let path = env::current_dir().unwrap().join(".cache");
            let project = param.get("project");
            let project_namespace = project.unwrap().get("namespace").unwrap().as_str().unwrap().to_string();
            let project_name = project.unwrap().get("name").unwrap().as_str().unwrap().to_string();
            let target_path = &path.join(&project_namespace).join(&project_name).join(&target_branch);

            match Self::run_build_script(msg.param.clone(), receiver.clone()) {
                Ok(()) => {
                    {
                        let run_task = run_task.lock().unwrap();
                        let binding = run_task.clone();
                        let mut run_task = binding.borrow_mut();
                        let position = run_task.iter().position(|item| item.eq(&ids));
                        if let Some(position) = position {
                            run_task.remove(position);
                        }
                    }
                    let content = fs::read_to_string(target_path.join(".pipeline.toml")).unwrap();
                    let setting: PipelineToml  = toml::from_str(&content).unwrap();
                    
                    for (name, item) in setting.publish.git.iter() {

                        let output_directory = target_path.join(&setting.build.output_directory).join(".git");
                        if  output_directory.exists() {
                            std::fs::remove_dir_all(output_directory).unwrap();
                        }

                        let mut new_branch_name = item.branch.clone();

                        if new_branch_name.eq("$auto_branch") {
                            new_branch_name = format!("RCD_{}", target_branch);
                        }

                        let command = match &item.ssh_key  {
                            Some(git_ssh_command) => {
                                format!(
                                    "GIT_SSH_COMMAND='ssh -i {}' && git init && git add . && git commit -am 'Rust Publish Static Files' && git checkout -b {} && git remote add {} {} && git push --force {} {} ",
                                    git_ssh_command,
                                    &new_branch_name,
                                    &name,
                                    &item.repository,
                                    &name,
                                    &new_branch_name,
                                )
                            },
                            None => {
                                format!(
                                    "git init && git add . && git commit -am 'Rust Publish Static Files' && git checkout -b {} && git remote add {} {} && git push --force {} {} ",
                                    &new_branch_name,
                                    &name,
                                    &item.repository,
                                    &name,
                                    &new_branch_name,
                                )
                            }
                        };

                        info!("run command -> {} ", &command);
                        let child = Command::new("sh")
                        .current_dir(&target_path.join(&setting.build.output_directory))
                        .arg("-c")
                        .arg(command)
                        .stdout(Stdio::piped())
                        .spawn().unwrap();
                        child.wait_with_output().unwrap();
                    }
                },
                Err(err) => {
                    let run_task = run_task.lock().unwrap();
                    let binding = run_task.clone();
                    let mut run_task = binding.borrow_mut();
                    let position = run_task.iter().position(|item| item.eq(&ids));
                    if let Some(position) = position {
                        run_task.remove(position);
                    }
                    error!("ERR -> {}", err.to_string());
                }
            } 
        });
    }
}


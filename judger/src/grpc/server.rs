// TODO: clean up imports
// TODO: error handling
use std::{pin::Pin, sync::Arc};

use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{codegen::Bytes, metadata, Code, Response, Status};
use uuid::Uuid;

use crate::{
    grpc::proto::prelude::judge_response,
    init::config::CONFIG,
    langs::{
        prelude::{ArtifactFactory, CompileLog, Error as LangError},
        RequestError,
    },
};

use super::proto::prelude::{judger_server::Judger, *};

pub type UUID = String;

fn accuracy() -> u64 {
    let config = CONFIG.get().unwrap();
    (1000 * 1000 / config.kernel.kernel_hz) as u64
}

impl From<LangError> for Result<JudgeResponse, Status> {
    fn from(value: LangError) -> Self {
        match value {
            LangError::Internal(err) => {
                log::warn!("{}", err);
                #[cfg(debug_assertions)]
                return Err(Status::with_details(
                    Code::Internal,
                    "Lanuage internal error: see debug info",
                    Bytes::from(format!("{}", err)),
                ));
                #[cfg(not(debug_assertions))]
                Err(Status::internal("See log for more details"))
            }
            LangError::BadRequest(err) => match err {
                RequestError::LangNotFound(uid) => Err(Status::with_details(
                    Code::FailedPrecondition,
                    "language with such uuid does not exist on this judger",
                    Bytes::from(format!("lang_uid: {}", uid)),
                )),
            },
            LangError::Report(res) => Ok(JudgeResponse {
                task: Some(judge_response::Task::Result(JudgeResult {
                    status: res as i32,
                    time: 0,
                    memory: 0,
                    accuracy: accuracy(),
                })),
            }),
        }
    }
}

macro_rules! report {
    ($result:expr,$tx:expr) => {
        match $result {
            Ok(x) => x,
            Err(err) => {
                $tx.send(err.into()).await.ok();
                return ();
            }
        }
    };
}

macro_rules! resud {
    ($result:expr) => {
        match $result {
            Ok(x) => x,
            Err(err) => {
                log::trace!("{}", err);
                return;
            }
        }
    };
}

impl From<CompileLog<'_>> for Log {
    fn from(value: CompileLog<'_>) -> Self {
        Log {
            level: value.level as u32,
            msg: value.message.into_owned(),
        }
    }
}

// Adapter and abstraction for tonic to serve
// utilize artifact factory and other components(in module `langs``)
pub struct Server {
    factory: Arc<ArtifactFactory>,
}

impl Server {
    pub async fn new() -> Self {
        let config = CONFIG.get().unwrap();
        let mut factory = ArtifactFactory::default();

        factory.load_dir(config.plugin.path.clone()).await;

        Self {
            factory: Arc::new(factory),
        }
    }
}

#[tonic::async_trait]
impl Judger for Server {
    type JudgeStream = Pin<Box<dyn futures::Stream<Item = Result<JudgeResponse, Status>> + Send>>;

    async fn judge<'a>(
        &'a self,
        request: tonic::Request<JudgeRequest>,
    ) -> Result<Response<Self::JudgeStream>, Status> {
        let (meta, _, request) = request.into_parts();
        check_secret(&meta)?;

        let (tx, rx) = mpsc::channel(2);

        let factory = self.factory.clone();

        // precondidtion
        let mode = JudgeMatchRule::from_i32(request.rule)
            .ok_or(Status::failed_precondition("Invaild judge matching rule"))?;

        let lang_uid = Uuid::parse_str(request.lang_uid.as_str()).map_err(|e| {
            log::warn!("Invalid uuid: {}", e);
            Status::failed_precondition("Invalid uuid")
        })?;

        tokio::spawn(async move {
            let time = request.time;
            let memory = request.memory;

            let mut compiled = report!(factory.compile(&lang_uid, &request.code).await, tx);

            let mut running_task = 1;

            for task in request.tests {
                tx.send(Ok(JudgeResponse {
                    task: Some(judge_response::Task::Case(running_task)),
                }))
                .await
                .ok();

                let result = report!(compiled.judge(&task.input, time, memory).await, tx);

                if let Some(x) = result.get_expection() {
                    tx.send(Ok(JudgeResponse {
                        task: Some(judge_response::Task::Result(JudgeResult {
                            status: x as i32,
                            time: result.time().total_us,
                            memory: result.mem().peak,
                            accuracy: accuracy(),
                        })),
                    }))
                    .await
                    .ok();
                    return;
                }

                tx.send(Ok(JudgeResponse {
                    task: Some(judge_response::Task::Result(JudgeResult {
                        status: match result.assert(&task.output, mode) {
                            true => JudgerCode::Ac,
                            false => JudgerCode::Wa,
                        } as i32,
                        time: result.time().total_us,
                        memory: result.mem().peak,
                        accuracy: accuracy(),
                    })),
                }))
                .await
                .ok();
                running_task += 1;
            }
        });

        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }
    async fn judger_info<'a>(
        &'a self,
        request: tonic::Request<()>,
    ) -> Result<Response<JudgeInfo>, Status> {
        let config = CONFIG.get().unwrap();

        let (meta, _, _) = request.into_parts();
        check_secret(&meta)?;

        let modules = self.factory.list_module();

        Ok(Response::new(JudgeInfo {
            langs: Langs { list: modules },
            memory: config.platform.available_memory,
            accuracy: accuracy(),
            cpu_factor: config.platform.cpu_time_multiplier as f32,
        }))
    }

    #[doc = " Server streaming response type for the Exec method."]
    type ExecStream = Pin<Box<dyn futures::Stream<Item = Result<ExecResult, Status>> + Send>>;

    async fn exec(
        &self,
        req: tonic::Request<ExecRequest>,
    ) -> Result<Response<Self::ExecStream>, tonic::Status> {
        let (meta, _, payload) = req.into_parts();
        check_secret(&meta)?;

        let (tx, rx) = mpsc::channel(2);

        let factory = self.factory.clone();

        let lang_uid = Uuid::parse_str(payload.lang_uid.as_str()).map_err(|e| {
            log::warn!("Invalid uuid: {}", e);
            Status::failed_precondition("Invalid uuid")
        })?;

        tokio::spawn(async move {
            let input = payload.input;
            let time = payload.time;
            let memory = payload.memory;

            let mut compiled = resud!(factory.compile(&lang_uid, &payload.code).await);

            while let Some(x) = compiled
                .to_log()
                .map(|x| ExecResult {
                    result: Some(exec_result::Result::Log(x.into())),
                })
                .next()
            {
                resud!(tx.send(Ok(x)).await);
            }

            let exec = resud!(compiled.exec(&input, time, memory).await);

            resud!(
                tx.send(Ok(ExecResult {
                    result: Some(exec_result::Result::Output(exec.stdout().to_vec()))
                }))
                .await
            );
        });

        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }
}

fn check_secret(meta: &metadata::MetadataMap) -> Result<(), Status> {
    let config = CONFIG.get().unwrap();
    if config.secret.is_none() {
        return Ok(());
    }
    if let Some(header) = meta.get("Authorization") {
        let secret = ["basic ", config.secret.as_ref().unwrap()]
            .concat()
            .into_bytes();
        let vaild = header
            .as_bytes()
            .iter()
            .zip(secret.iter())
            .map(|(&a, &b)| a == b)
            .reduce(|a, b| a && b);
        if vaild.unwrap_or(false) {
            return Ok(());
        }
    }
    Err(Status::permission_denied("Invalid secret"))
}

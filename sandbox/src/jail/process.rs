use std::fmt::Display;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    select, time,
};

use crate::jail::utils::limiter::LimitReason;

use super::{
    utils::{
        limiter::{cpu::CpuStatistics, mem::MemStatistics, Limiter},
        nsjail::NsJail,
        preserve::MemoryHolder,
    },
    Error,
};

const BUFFER_LIMIT: usize = 32 * 1024 * 1024 - 1;

pub struct RunningProc {
    pub(super) limiter: Limiter,
    pub(super) nsjail: NsJail,
    pub(super) _memory_holder: MemoryHolder,
}

impl RunningProc {
    pub async fn write_all(&mut self, buf: &Vec<u8>) -> Result<(), Error> {
        let mut child = self.nsjail.process.as_ref().unwrap().lock().await;
        let stdin = child.stdin.as_mut().ok_or(Error::CapturedPipe)?;

        stdin.write_all(&buf).await?;

        Ok(())
    }
    pub async fn wait(mut self) -> Result<ExitProc, Error> {
        let status = select! {
            reason = self.limiter.wait_exhausted()=>{
                match reason.unwrap(){
                    LimitReason::Cpu=>ExitStatus::CpuExhausted,
                    LimitReason::Mem=>ExitStatus::MemExhausted
                }
            }
            code = self.nsjail.wait()=>{
                match code{
                    Some(x)=>ExitStatus::Code(x),
                    None=>ExitStatus::SigExit
                }
            }
            _ = time::sleep(time::Duration::from_secs(3600))=>{
                return Err(Error::Stall);
            }
        };

        let mut child = self.nsjail.process.as_ref().unwrap().lock().await;
        let stdout = child.stdout.as_mut().ok_or(Error::CapturedPipe)?;

        let mut buf = Vec::with_capacity(BUFFER_LIMIT + 1);

        let buffer_size = stdout
            .take((BUFFER_LIMIT + 1) as u64)
            .read_to_end(&mut buf)
            .await?;

        if buffer_size == BUFFER_LIMIT + 1 {
            return Err(Error::BufferFull);
        }

        let (cpu, mem) = self.limiter.status().await;

        Ok(ExitProc {
            status,
            stdout: buf.to_vec(),
            cpu,
            mem,
        })
    }
}

pub struct ExitProc {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub cpu: CpuStatistics,
    pub mem: MemStatistics,
}

impl ExitProc {
    pub fn succeed(&self) -> bool {
        match self.status {
            ExitStatus::Code(x) => x == 0,
            _ => false,
        }
    }
}

pub enum ExitStatus {
    SigExit, // RuntimeError
    Code(i32),
    MemExhausted,
    CpuExhausted,
}

impl Display for ExitStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExitStatus::SigExit => write!(f, "Killed by signal"),
            ExitStatus::Code(x) => write!(f, "Exit with code {}", x),
            ExitStatus::MemExhausted => write!(f, "Reach memory limit"),
            ExitStatus::CpuExhausted => write!(f, "Reach cpu quota"),
        }
    }
}

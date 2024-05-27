/// Error occurred in the filesystem adapter.
///
/// It's only used to manage the error in a centralized way.
///
/// User shouldn't rely on this error to as value in another error,
/// and should always call [`Into::<fuse3::Errno>>::into`]
/// immediately after the error is returned.
#[derive(thiserror::Error, Debug)]
pub enum FuseError {
    #[error("not a readable file")]
    IsDir,
    #[error("end of file")]
    Eof,
    #[error("not a dir")]
    NotDir,
    #[error("out of resource")]
    OutOfPermit,
    #[error("number too large")]
    OutOfRange,
    #[error("unimplemented")]
    Unimplemented,
    #[error("missed inode")]
    InvaildIno,
    #[error("missed handle")]
    HandleNotFound,
    #[error("underlaying file error")]
    Underlaying,
    #[error("invalid path")]
    InvalidPath,
}

impl From<FuseError> for fuse3::Errno {
    fn from(value: FuseError) -> Self {
        #[cfg(test)]
        log::warn!("FUSE driver return result: {}", value);
        match value {
            FuseError::IsDir => libc::EISDIR,
            FuseError::NotDir => libc::ENOTDIR,
            FuseError::Eof => libc::EOF,
            FuseError::OutOfPermit => {
                log::info!("out of resource");
                libc::ENOMEM
            }
            FuseError::InvalidPath | FuseError::InvaildIno => libc::ENOENT,
            _ => {
                log::warn!("FUSE driver broken: {}", value);
                libc::ENOMEM
            }
        }
        .into()
    }
}

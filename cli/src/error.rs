use std::path::PathBuf;

/// Failures the library reports; each maps to a stable exit code.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{path}: {message}")]
    InvalidSnapshot { path: String, message: String },
    #[error(
        "{path}: checksum mismatch, the part says {expected} but its data hashes to {actual}; copy the part again from the collector output"
    )]
    ChecksumMismatch {
        path: String,
        expected: String,
        actual: String,
    },
    #[error("{source_name} refers to variable {target}, which is not in the snapshot")]
    DanglingAlias { source_name: String, target: String },
    #[error("cannot read or write {path}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{scope}: {first:?} and {second:?} both become {name}; rename one of them in Figma")]
    DuplicateIdentifier {
        scope: String,
        name: String,
        first: String,
        second: String,
    },
    #[error("no local collection has a mode named {name:?}; available: {}", available.join(", "))]
    UnknownMode {
        name: String,
        available: Vec<String>,
    },
}

impl Error {
    /// Exit codes are append-only: 3 invalid snapshot, 4 checksum mismatch,
    /// 5 dangling alias, 6 I/O, 7 duplicate identifier, 8 unknown mode. Clap
    /// owns 2 for usage errors.
    pub fn exit_code(&self) -> u8 {
        match self {
            Error::InvalidSnapshot { .. } => 3,
            Error::ChecksumMismatch { .. } => 4,
            Error::DanglingAlias { .. } => 5,
            Error::Io { .. } => 6,
            Error::DuplicateIdentifier { .. } => 7,
            Error::UnknownMode { .. } => 8,
        }
    }

    /// Stable machine-readable name for the `--json` error envelope.
    pub fn kind(&self) -> &'static str {
        match self {
            Error::InvalidSnapshot { .. } => "invalid_snapshot",
            Error::ChecksumMismatch { .. } => "checksum_mismatch",
            Error::DanglingAlias { .. } => "dangling_alias",
            Error::Io { .. } => "io",
            Error::DuplicateIdentifier { .. } => "duplicate_identifier",
            Error::UnknownMode { .. } => "unknown_mode",
        }
    }
}

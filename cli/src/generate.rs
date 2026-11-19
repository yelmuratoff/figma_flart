use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::codegen::{self, Kind, Options};
use crate::error::Error;
use crate::snapshot::Snapshot;

/// What to generate from a snapshot.
#[derive(Debug, Clone)]
pub struct Request {
    pub kinds: Vec<Kind>,
    pub options: Options,
    /// Mode names; empty means the plugin's no-selection path.
    pub modes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Created,
    Changed,
    Unchanged,
    Skipped,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileOutcome {
    pub kind: &'static str,
    pub path: PathBuf,
    pub status: Status,
    /// Mode names this file was generated for; empty for the no-selection path.
    pub modes: Vec<String>,
}

struct Generated {
    kind: Kind,
    code: Option<String>,
    modes: Vec<String>,
}

/// Generates the requested kinds into `out/<kind>.dart`. With `dry_run` the
/// statuses are computed and nothing is written.
pub fn generate(
    snapshot: &Snapshot,
    request: &Request,
    out: &Path,
    dry_run: bool,
) -> Result<Vec<FileOutcome>, Error> {
    ensure_modes_exist(snapshot, &request.modes)?;
    let generated = request
        .kinds
        .iter()
        .map(|&kind| generate_kind(snapshot, request, kind))
        .collect::<Result<Vec<_>, _>>()?;

    if !dry_run {
        std::fs::create_dir_all(out).map_err(|source| Error::Io {
            path: out.to_path_buf(),
            source,
        })?;
    }
    generated
        .into_iter()
        .map(|file| {
            let path = out.join(format!("{}.dart", file.kind.file_stem()));
            let status = match &file.code {
                None => Status::Skipped,
                Some(code) => write_if_changed(&path, code, dry_run)?,
            };
            Ok(FileOutcome {
                kind: file.kind.file_stem(),
                path,
                status,
                modes: file.modes,
            })
        })
        .collect()
}

fn ensure_modes_exist(snapshot: &Snapshot, names: &[String]) -> Result<(), Error> {
    let mut available: Vec<String> = Vec::new();
    for collection in snapshot.collections.iter().filter(|c| !c.remote) {
        for mode in &collection.modes {
            if !available.contains(&mode.name) {
                available.push(mode.name.clone());
            }
        }
    }
    match names.iter().find(|name| !available.contains(name)) {
        Some(name) => Err(Error::UnknownMode {
            name: name.clone(),
            available,
        }),
        None => Ok(()),
    }
}

fn generate_kind(snapshot: &Snapshot, request: &Request, kind: Kind) -> Result<Generated, Error> {
    if !kind.is_present_in(snapshot) {
        return Ok(Generated {
            kind,
            code: None,
            modes: Vec::new(),
        });
    }
    let chosen: Vec<_> = kind
        .selectable_modes(snapshot)
        .into_iter()
        .filter(|mode| request.modes.contains(&mode.name))
        .collect();
    let ids: Vec<String> = chosen.iter().map(|mode| mode.mode_id.clone()).collect();
    let selected = (!ids.is_empty()).then_some(ids.as_slice());
    let code = codegen::generate(kind, snapshot, request.options, selected)?;
    Ok(Generated {
        kind,
        code: (!code.is_empty()).then_some(code),
        modes: chosen.into_iter().map(|mode| mode.name).collect(),
    })
}

fn write_if_changed(path: &Path, code: &str, dry_run: bool) -> Result<Status, Error> {
    let status = match std::fs::read_to_string(path) {
        Ok(existing) if existing == code => return Ok(Status::Unchanged),
        Ok(_) => Status::Changed,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Status::Created,
        Err(source) => {
            return Err(Error::Io {
                path: path.to_path_buf(),
                source,
            });
        }
    };
    if !dry_run {
        std::fs::write(path, code).map_err(|source| Error::Io {
            path: path.to_path_buf(),
            source,
        })?;
    }
    Ok(status)
}

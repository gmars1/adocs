use serde::{Deserialize, Serialize};

use super::ledger::{DocEvidence, SealEvidence};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustState {
    Stale,
    Valid,
    Sealed,
}

impl std::fmt::Display for TrustState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrustState::Stale => write!(f, "stale"),
            TrustState::Valid => write!(f, "valid"),
            TrustState::Sealed => write!(f, "sealed"),
        }
    }
}

pub fn file_state(
    current_sha256: &str,
    doc: Option<&DocEvidence>,
    seal: Option<&SealEvidence>,
    description_exists: bool,
) -> TrustState {
    let Some(doc) = doc else {
        return TrustState::Stale;
    };

    if !description_exists || doc.accepted_source_sha256 != current_sha256 {
        return TrustState::Stale;
    }

    match seal {
        Some(seal) if seal.source_sha256 == current_sha256 => TrustState::Sealed,
        _ => TrustState::Valid,
    }
}

pub fn folder_purpose_state(
    folder_doc_exists: bool,
    doc: Option<&DocEvidence>,
    current_doc_sha256: Option<&str>,
    seal: Option<&SealEvidence>,
) -> TrustState {
    if !folder_doc_exists {
        return TrustState::Stale;
    }

    let Some(doc) = doc else {
        return TrustState::Stale;
    };

    let current_hash = match current_doc_sha256 {
        Some(h) => h,
        None => return TrustState::Stale,
    };

    if doc.accepted_source_sha256 != current_hash {
        return TrustState::Stale;
    }

    match seal {
        Some(seal) if seal.source_sha256 == current_hash => TrustState::Sealed,
        _ => TrustState::Valid,
    }
}

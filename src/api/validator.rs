//! Custom validators
//!
//! Used to validate json data from clients

use std::{borrow::Cow, collections::HashSet};
use validator::ValidationError;

/// Check whether str is one of the list
fn oneof(item: &str, list: &Vec<&str>) -> Result<(), ValidationError> {
    if list.contains(&item) {
        Ok(())
    } else {
        let mut e = ValidationError::new("oneof");
        let msg = format!("must be one of {}", list.join(","));
        e.message = Some(Cow::from(msg));
        Err(e)
    }
}

pub fn validate_user_role(role: &str) -> Result<(), ValidationError> {
    let roles = vec!["admin", "user"];
    oneof(role, &roles)
}

#[allow(dead_code)]
pub fn validate_friend_status(status: &str) -> Result<(), ValidationError> {
    let status_vec = vec!["adding", "accepted", "deleted"];
    oneof(status, &status_vec)
}

#[allow(dead_code)]
pub fn validate_room_category(category: &str) -> Result<(), ValidationError> {
    let categories = vec!["public", "private", "personal"];
    oneof(category, &categories)
}

#[allow(dead_code)]
pub fn validate_room_rank(rank: &str) -> Result<(), ValidationError> {
    let ranks = vec!["owner", "manager", "member"];
    oneof(rank, &ranks)
}

pub fn validate_message_kind(kind: &str) -> Result<(), ValidationError> {
    let kinds = vec!["text", "file"];
    oneof(kind, &kinds)
}

pub fn validate_id_vec(ids: &Vec<i64>) -> Result<(), ValidationError> {
    let mut seen = HashSet::new();
    for &id in ids {
        if id < 1 || !seen.insert(id) {
            let mut e = ValidationError::new("vec");
            let msg = format!("must be greater than 0 and not contain duplicate numbers");
            e.message = Some(Cow::from(msg));
            return Err(e);
        }
    }

    Ok(())
}

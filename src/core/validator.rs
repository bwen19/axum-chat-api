//! Custom validators

use super::constant::{
    CATEGORY_PERSONAL, CATEGORY_PRIVATE, CATEGORY_PUBLIC, KIND_FILE, KIND_TEXT, RANK_MANAGER,
    RANK_MEMBER, RANK_OWNER, ROLE_ADMIN, ROLE_USER, STATUS_ACCEPTED, STATUS_ADDING, STATUS_DELETED,
};
use std::{borrow::Cow, collections::HashSet};
use validator::ValidationError;

pub fn validate_user_role(role: &str) -> Result<(), ValidationError> {
    let roles = vec![ROLE_ADMIN, ROLE_USER];
    validate_oneof(role, &roles)
}

#[allow(dead_code)]
pub fn validate_friend_status(status: &str) -> Result<(), ValidationError> {
    let status_vec = vec![STATUS_ADDING, STATUS_ACCEPTED, STATUS_DELETED];
    validate_oneof(status, &status_vec)
}

#[allow(dead_code)]
pub fn validate_room_category(category: &str) -> Result<(), ValidationError> {
    let categories = vec![CATEGORY_PUBLIC, CATEGORY_PRIVATE, CATEGORY_PERSONAL];
    validate_oneof(category, &categories)
}

#[allow(dead_code)]
pub fn validate_room_rank(rank: &str) -> Result<(), ValidationError> {
    let ranks = vec![RANK_OWNER, RANK_MANAGER, RANK_MEMBER];
    validate_oneof(rank, &ranks)
}

pub fn validate_message_kind(kind: &str) -> Result<(), ValidationError> {
    let kinds = vec![KIND_TEXT, KIND_FILE];
    validate_oneof(kind, &kinds)
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

/// Check whether str is one of the list
fn validate_oneof(item: &str, list: &Vec<&str>) -> Result<(), ValidationError> {
    if list.contains(&item) {
        Ok(())
    } else {
        let mut e = ValidationError::new("validate_oneof");
        let msg = format!("must be one of {}", list.join(","));
        e.message = Some(Cow::from(msg));
        Err(e)
    }
}

use super::Store;
use crate::api::invitation::{CreateInvitationRequest, ListInvitationsRequest};
use crate::db::model::Invitation;
use crate::error::ResultExt;
use crate::{
    error::{AppError, AppResult},
    util,
};
use chrono::{DateTime, Days, Utc};

// ========================// Invitation Store //======================== //

impl Store {
    /// Create a new invitation with the given params
    pub async fn create_invitation(&self, arg: &CreateInvitationParam) -> AppResult<Invitation> {
        let result = sqlx::query_as!(
            Invitation,
            r#"
                INSERT INTO invitations (code, expire_at)
                VALUES ($1, $2)
                ON CONFLICT (code)
                DO UPDATE SET expire_at = $2
                RETURNING code, expire_at
            "#,
            arg.code,
            arg.expire_at,
        )
        .fetch_one(&self.pool)
        .await
        .exactly_one()?;

        Ok(result)
    }

    /// Delete a list of users by IDs
    pub async fn delete_invitations(&self, codes: &Vec<String>) -> AppResult<()> {
        sqlx::query!(
            r#"
                DELETE FROM invitations
                WHERE code = ANY($1::varchar[])
            "#,
            codes
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get a session by session ID
    pub async fn validate_invitation(&self, code: &String) -> AppResult<()> {
        let result = sqlx::query_as!(
            Invitation,
            r#"
                SELECT code, expire_at
                FROM invitations
                WHERE code = $1
            "#,
            code
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(invitation) = result {
            if invitation.expire_at > Utc::now() {
                return Ok(());
            }
        }
        Err(AppError::InvitationCode)
    }

    /// Get a list of invitation's information
    pub async fn list_invitations(
        &self,
        arg: &ListInvitationsParam,
    ) -> AppResult<(i64, Vec<Invitation>)> {
        let result = sqlx::query_as!(
            ListInvitationsRow,
            r#"
                SELECT code, expire_at, count(*) OVER() AS total
                FROM invitations
                LIMIT $1
                OFFSET $2
            "#,
            arg.limit,
            arg.offset,
        )
        .fetch_all(&self.pool)
        .await?;

        let total = match result.get(0) {
            Some(row) => row.total.unwrap_or(0),
            None => 0,
        };
        let arr: Vec<Invitation> = result.into_iter().map(|row| row.into()).collect();

        Ok((total, arr))
    }
}

// ========================// Conversions //======================== //

pub struct CreateInvitationParam {
    pub code: String,
    pub expire_at: DateTime<Utc>,
}

impl From<CreateInvitationRequest> for CreateInvitationParam {
    fn from(value: CreateInvitationRequest) -> Self {
        let code = util::common::random_string(value.length);
        let expire_at = Utc::now() + Days::new(value.days);

        Self { code, expire_at }
    }
}

pub struct ListInvitationsParam {
    limit: i64,
    offset: i64,
}

impl From<ListInvitationsRequest> for ListInvitationsParam {
    fn from(req: ListInvitationsRequest) -> Self {
        let page_id = req.page_id.unwrap_or(1);
        let page_size = req.page_size.unwrap_or(10);

        Self {
            limit: page_size,
            offset: (page_id - 1) * page_size,
        }
    }
}

struct ListInvitationsRow {
    code: String,
    expire_at: DateTime<Utc>,
    total: Option<i64>,
}

impl From<ListInvitationsRow> for Invitation {
    fn from(value: ListInvitationsRow) -> Self {
        Self {
            code: value.code,
            expire_at: value.expire_at,
        }
    }
}

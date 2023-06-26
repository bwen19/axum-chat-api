use super::Store;
use crate::api::member::{
    AddMembersRequest, AddMembersResponse, DeleteMembersRequest, DeleteMembersResponse,
};
use crate::db::model::MemberInfo;
use crate::error::AppResult;

// ========================// Member Store //======================== //

impl Store {
    /// Add members into the room and return these members info
    pub async fn add_members(&self, arg: &AddMembersRequest) -> AppResult<AddMembersResponse> {
        let room_ids = vec![arg.room_id; arg.member_ids.len()];

        let mid = sqlx::query_scalar!(
            r#"
                INSERT INTO room_members (room_id, member_id)
                SELECT * FROM UNNEST($1::bigint[], $2::bigint[])
                ON CONFLICT (room_id, member_id) DO NOTHING
                RETURNING member_id
            "#,
            &room_ids,
            &arg.member_ids,
        )
        .fetch_all(&self.pool)
        .await?;

        let members = sqlx::query_as!(
            MemberInfo,
            r#"
                SELECT id, nickname AS name, avatar, rank, y.join_at
                FROM room_members y
                INNER JOIN users u ON y.member_id = u.id
                WHERE y.room_id = $1 AND y.member_id = ANY($2::bigint[])
            "#,
            &arg.room_id,
            &mid,
        )
        .fetch_all(&self.pool)
        .await
        .map(|records| records.into_iter().map(|x| x.into()).collect())?;

        Ok(AddMembersResponse {
            room_id: arg.room_id,
            members,
        })
    }

    /// Delete room members
    ///
    /// The owner can not be deleted, otherwise the room will be deleted
    pub async fn delete_members(
        &self,
        arg: &DeleteMembersRequest,
    ) -> AppResult<DeleteMembersResponse> {
        let member_ids = sqlx::query_scalar!(
            r#"
                DELETE FROM room_members
                WHERE room_id = $1 AND member_id = ANY($2::bigint[]) AND rank <> 'owner'
                RETURNING member_id
            "#,
            arg.room_id,
            &arg.member_ids,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(DeleteMembersResponse {
            room_id: arg.room_id,
            member_ids,
        })
    }

    /// Check whether the rank of a member meets the requirements
    pub async fn check_rank(&self, member_id: i64, room_id: i64, desired: &str) -> AppResult<bool> {
        let rank = sqlx::query_scalar!(
            r#"
                SELECT rank
                FROM room_members
                WHERE room_id = $1 AND member_id = $2
            "#,
            room_id,
            member_id,
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(rank) = rank {
            let pass = match desired {
                "owner" => rank == "owner",
                "manager" => rank == "owner" || rank == "manager",
                _ => true,
            };
            Ok(pass)
        } else {
            Ok(false)
        }
    }
}

use super::Store;
use crate::{
    api::{AddMembersRequest, DeleteMembersRequest},
    core::{constant::RANK_MEMBER, Error, ResultExt},
    store::MemberInfo,
};

impl Store {
    /// Add members into the room and return these members info
    pub async fn add_members(&self, req: &AddMembersRequest) -> Result<Vec<MemberInfo>, Error> {
        let rooms_id = vec![req.room_id; req.members_id.len()];
        let ranks = vec![RANK_MEMBER.to_string(); req.members_id.len()];

        let members_id = sqlx::query_scalar!(
            r#"
                INSERT INTO members
                    (room_id, member_id, rank)
                SELECT * FROM
                    UNNEST($1::bigint[], $2::bigint[], $3::varchar[])
                ON CONFLICT (room_id, member_id) DO NOTHING
                RETURNING member_id
            "#,
            &rooms_id,
            &req.members_id,
            &ranks,
        )
        .fetch_all(&self.pool)
        .await?;

        let members = sqlx::query_as!(
            MemberInfo,
            r#"
                SELECT
                    id, nickname AS name, avatar, rank, y.join_at
                FROM members y
                JOIN users u ON y.member_id = u.id
                WHERE
                    y.room_id = $1
                    AND y.member_id = ANY($2::bigint[])
            "#,
            &req.room_id,
            &members_id,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(members)
    }

    /// Delete room members
    ///
    /// The owner can not be deleted, otherwise the room will be deleted
    pub async fn delete_members(&self, req: &DeleteMembersRequest) -> Result<Vec<i64>, Error> {
        let members_id = sqlx::query_scalar!(
            r#"
                DELETE FROM members
                WHERE
                    room_id = $1
                    AND member_id = ANY($2::bigint[])
                    AND rank <> 'owner'
                RETURNING member_id
            "#,
            req.room_id,
            &req.members_id,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(members_id)
    }

    /// Get the rank of a member in the room
    pub async fn get_rank(&self, member_id: i64, room_id: i64) -> Result<String, Error> {
        sqlx::query_scalar!(
            r#"
                SELECT rank
                FROM members
                WHERE
                    room_id = $1
                    AND member_id = $2
            "#,
            room_id,
            member_id,
        )
        .fetch_one(&self.pool)
        .await
        .not_found()
    }
}

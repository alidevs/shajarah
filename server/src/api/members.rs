use std::sync::Arc;

use anyhow::anyhow;
use axum::{extract::State, Json};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{AppError, AppState, Gender};

#[derive(Deserialize, Serialize)]
pub struct NewMember {
    name: String,
    last_name: String,
    gender: Gender,
    birthday: chrono::DateTime<chrono::Utc>,
    mother_id: Option<i32>,
    father_id: Option<i32>,
}

#[allow(dead_code)]
#[derive(Debug, sqlx::FromRow)]
struct MemberRow {
    id: i32,
    name: String,
    gender: Gender,
    birthday: Option<chrono::DateTime<chrono::Utc>>,
    last_name: String,
    mother_id: Option<i32>,
    mother_name: Option<String>,
    mother_gender: Option<Gender>,
    mother_birthday: Option<chrono::DateTime<chrono::Utc>>,
    mother_last_name: Option<String>,
    father_id: Option<i32>,
    father_name: Option<String>,
    father_gender: Option<Gender>,
    father_birthday: Option<chrono::DateTime<chrono::Utc>>,
    father_last_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MemberResponse {
    id: i32,
    name: String,
    gender: Gender,
    birthday: Option<DateTime<Utc>>,
    last_name: String,
    father_id: Option<i32>,
    mother_id: Option<i32>,
    children: Vec<MemberResponse>,
}

impl MemberResponse {
    fn add_children(&mut self, all_members: &Vec<MemberRow>) {
        self.children = all_members
            .iter()
            .filter(|m| {
                m.father_id.is_some_and(|fid| fid == self.id)
                    || m.mother_id.is_some_and(|mid| mid == self.id)
            })
            .map(|m| MemberResponse {
                id: m.id,
                name: m.name.clone(),
                gender: m.gender,
                birthday: m.birthday,
                last_name: m.last_name.clone(),
                father_id: m.father_id,
                mother_id: m.mother_id,
                children: vec![],
            })
            .collect();
        for child in &mut self.children {
            child.add_children(all_members);
        }
    }
}

/// Get family members
#[axum::debug_handler]
pub async fn get_members(
    State(state): State<Arc<AppState>>,
) -> anyhow::Result<Json<MemberResponse>, AppError> {
    let recs = sqlx::query_as!(
        MemberRow,
        r#"
SELECT
    m.id,
    m.name,
    m.gender as "gender: Gender",
    m.birthday,
    m.last_name,
    mother.id AS mother_id,
    mother.name AS mother_name,
    mother.gender AS "mother_gender: Gender",
    mother.birthday AS mother_birthday,
    mother.last_name AS mother_last_name,
    father.id AS father_id,
    father.name AS father_name,
    father.gender AS "father_gender: Gender",
    father.birthday AS father_birthday,
    father.last_name AS father_last_name
FROM
    members m
LEFT JOIN
    members mother ON m.mother_id = mother.id
LEFT JOIN
    members father ON m.father_id = father.id;
    "#,
    )
    .fetch_all(&state.db_pool)
    .await?;

    if recs.is_empty() {
        return Err(anyhow!("no records").into());
    }

    let Some(root) = recs
        .iter()
        .find(|rec| rec.father_id.is_none() && rec.mother_id.is_none())
    else {
        return Err(anyhow!("no root node").into());
    };

    let mut root = MemberResponse {
        id: root.id,
        name: root.name.clone(),
        gender: root.gender,
        birthday: root.birthday,
        last_name: root.last_name.clone(),
        father_id: None,
        mother_id: None,
        children: Vec::new(),
    };

    root.add_children(&recs);

    Ok(Json(root))
}

/// Add a family member
#[axum::debug_handler]
pub async fn add_member(
    State(state): State<Arc<AppState>>,
    Json(member): Json<NewMember>,
) -> anyhow::Result<(), AppError> {
    let _rec = sqlx::query!(
        r#"
INSERT INTO members (name, gender, birthday, last_name, father_id, mother_id)
VALUES ($1, $2, $3, $4, $5, $6)
RETURNING id, name, gender as "gender: Gender", birthday, mother_id, father_id, last_name
        "#,
        member.name,
        member.gender as _,
        member.birthday,
        member.last_name,
        member.father_id,
        member.mother_id,
    )
    .fetch_one(&state.db_pool)
    .await?;

    Ok(())
}

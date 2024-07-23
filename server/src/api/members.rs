use std::{collections::HashMap, sync::Arc};

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
    mother_id: Option<i64>,
    father_id: Option<i64>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct MemberResponse {
    id: i64,
    name: String,
    gender: Gender,
    birthday: Option<DateTime<Utc>>,
    last_name: String,
    children: Vec<MemberResponse>,
}

/// Get family members
#[axum::debug_handler]
pub async fn get_members(
    State(state): State<Arc<AppState>>,
) -> anyhow::Result<Json<MemberResponse>, AppError> {
    let recs = sqlx::query!(
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

    let mut members = HashMap::new();
    let mut parent_child_relations = Vec::new();

    for rec in recs.iter() {
        let member = MemberResponse {
            id: rec.id,
            name: rec.name.clone(),
            gender: rec.gender.unwrap(),
            birthday: rec.birthday,
            last_name: rec.last_name.clone(),
            children: Vec::new(),
        };

        members.insert(member.id, member);

        if let Some(mother_id) = rec.mother_id {
            if mother_id != rec.id {
                parent_child_relations.push((mother_id, rec.id));
            }
        }

        if let Some(father_id) = rec.father_id {
            if father_id != rec.id {
                parent_child_relations.push((father_id, rec.id));
            }
        }
    }

    for (parent_id, child_id) in parent_child_relations {
        if let Some(child) = members.remove(&child_id) {
            if let Some(parent) = members.get_mut(&parent_id) {
                parent.children.push(child);
            }
        }
    }

    let root_members: Vec<MemberResponse> = members
        .values()
        .filter(|member| {
            !members
                .values()
                .any(|parent| parent.children.iter().any(|child| child.id == member.id))
        })
        .cloned()
        .collect();

    Ok(Json(root_members.get(0).unwrap().clone()))
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

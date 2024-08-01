use std::sync::Arc;

use anyhow::anyhow;
use axum::{
    extract::{Multipart, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{auth::AuthExtractor, users::models::UserRole, AppError, Gender, InnerAppState};

const FIELDS_LIMIT: i32 = 10;

#[derive(Deserialize, Serialize)]
pub struct CreateMember {
    name: String,
    last_name: String,
    gender: Gender,
    birthday: chrono::DateTime<chrono::Utc>,
    mother_id: Option<i32>,
    father_id: Option<i32>,
    image: Option<Vec<u8>>,
}

#[derive(Default)]
pub struct CreateMemberBuilder {
    name: Option<String>,
    last_name: Option<String>,
    gender: Option<Gender>,
    birthday: Option<chrono::DateTime<chrono::Utc>>,
    mother_id: Option<i32>,
    father_id: Option<i32>,
    image: Option<Vec<u8>>,
}

impl CreateMemberBuilder {
    fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    fn name(&mut self, name: String) -> &mut Self {
        self.name = Some(name);
        self
    }

    fn last_name(&mut self, last_name: String) -> &mut Self {
        self.last_name = Some(last_name);
        self
    }

    fn gender(&mut self, gender: Gender) -> &mut Self {
        self.gender = Some(gender);
        self
    }

    fn birthday(&mut self, birthday: chrono::DateTime<chrono::Utc>) -> &mut Self {
        self.birthday = Some(birthday);
        self
    }

    fn mother_id(&mut self, mother_id: i32) -> &mut Self {
        self.mother_id = Some(mother_id);
        self
    }

    fn father_id(&mut self, father_id: i32) -> &mut Self {
        self.father_id = Some(father_id);
        self
    }

    fn image(&mut self, image: Vec<u8>) -> &mut Self {
        self.image = Some(image);
        self
    }

    fn build(self) -> Result<CreateMember, AppError> {
        let name = self.name.ok_or(anyhow!("name field was not provided"))?;
        let last_name = self
            .last_name
            .ok_or(anyhow!("last_name field was not provided"))?;
        let gender = self
            .gender
            .ok_or(anyhow!("gender field was not provided"))?;
        let birthday = self
            .birthday
            .ok_or(anyhow!("birthday field was not provided"))?;

        Ok(CreateMember {
            name,
            last_name,
            gender,
            birthday,
            mother_id: self.mother_id,
            father_id: self.father_id,
            image: self.image,
        })
    }
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
    fn add_all_children(&mut self, all_members: &Vec<MemberRow>) {
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
            child.add_all_children(all_members);
        }
    }
}

/// non-recursive MemberResponse
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MemberResponseBrief {
    id: i32,
    name: String,
    gender: Gender,
    birthday: Option<DateTime<Utc>>,
    last_name: String,
    father_id: Option<i32>,
    mother_id: Option<i32>,
}

/// Get family members
#[axum::debug_handler]
pub async fn get_members(
    State(state): State<Arc<InnerAppState>>,
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

    root.add_all_children(&recs);

    Ok(Json(root))
}

/// Get family members as a flat vector
#[axum::debug_handler]
pub async fn get_members_flat(
    State(state): State<Arc<InnerAppState>>,
) -> anyhow::Result<Json<Vec<MemberResponseBrief>>, AppError> {
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

    let members = recs
        .into_iter()
        .map(|r| MemberResponseBrief {
            id: r.id,
            name: r.name,
            gender: r.gender,
            birthday: r.birthday,
            last_name: r.last_name,
            father_id: r.father_id,
            mother_id: r.mother_id,
        })
        .collect();

    Ok(Json(members))
}

/// Add a family member
pub async fn add_member(
    _auth: AuthExtractor<{ UserRole::Admin as u8 }>,
    State(state): State<Arc<InnerAppState>>,
    mut multipart: Multipart,
) -> anyhow::Result<Json<i32>, AppError> {
    let mut limit = FIELDS_LIMIT;
    let mut create_member_builder = CreateMemberBuilder::new();

    while let Some(field) = multipart.next_field().await.unwrap() {
        match field.name() {
            Some("name") => {
                let Ok(name) = field.text().await else {
                    return Err(anyhow::anyhow!("invalid name value").into());
                };
                create_member_builder.name(name);
            }
            Some("last_name") => {
                let Ok(last_name) = field.text().await else {
                    return Err(anyhow::anyhow!("invalid last_name value").into());
                };
                create_member_builder.last_name(last_name);
            }
            Some("gender") => {
                let Ok(gender) = field.text().await else {
                    return Err(anyhow::anyhow!("invalid gender value").into());
                };
                if gender.to_lowercase() == "male" {
                    create_member_builder.gender(Gender::Male);
                } else if gender.to_lowercase() == "female" {
                    create_member_builder.gender(Gender::Female);
                } else {
                    return Err(anyhow!("invalid gender value").into());
                }
            }
            Some("birthday") => {
                let Ok(birthday) = field.text().await else {
                    return Err(anyhow::anyhow!("invalid birthday value").into());
                };
                let birthday = DateTime::parse_from_rfc2822(&birthday)
                    .or(DateTime::parse_from_rfc3339(&birthday))
                    .map_err(|e| {
                        log::error!("{e}");
                        anyhow!("wrong birthday format")
                    })?;
                create_member_builder.birthday(birthday.to_utc());
            }
            Some("father_id") => {
                let Ok(father_id) = field.text().await else {
                    return Err(anyhow::anyhow!("invalid father_id value").into());
                };
                create_member_builder.father_id(father_id.parse().map_err(|e| {
                    log::error!("{e}");
                    anyhow!("invalid father_id value")
                })?);
            }
            Some("mother_id") => {
                let Ok(mother_id) = field.text().await else {
                    return Err(anyhow::anyhow!("invalid mother_id value").into());
                };
                create_member_builder.mother_id(mother_id.parse().map_err(|e| {
                    log::error!("{e}");
                    anyhow!("invalid mother_id value")
                })?);
            }
            Some("image") => {
                if let Some(image_content_type) = field.content_type() {
                    match image_content_type {
                        "image/png" | "image/jpg" | "image/jpeg" => {
                            let Ok(image) = field.bytes().await else {
                                return Err(anyhow::anyhow!("invalid image value").into());
                            };
                            create_member_builder.image(image.to_vec());
                        }
                        _ => {
                            return Err(anyhow!("invalid image content-type").into());
                        }
                    }
                } else {
                    return Err(anyhow!("invalid image content-type").into());
                }
            }
            Some(field) => {
                return Err(anyhow!("invalid field name {field}").into());
            }
            None => {
                return Err(anyhow!("invalid request").into());
            }
        }
        if limit > 0 {
            limit -= 1;
        } else {
            break;
        }
    }

    let create_member = create_member_builder.build()?;

    let rec = sqlx::query!(
        r#"
    INSERT INTO members (name, gender, birthday, last_name, father_id, mother_id, image)
    VALUES ($1, $2, $3, $4, $5, $6, $7)
    RETURNING id, name, gender as "gender: Gender", birthday, mother_id, father_id, last_name
            "#,
        create_member.name,
        create_member.gender as _,
        create_member.birthday,
        create_member.last_name,
        create_member.father_id,
        create_member.mother_id,
        create_member.image,
    )
    .fetch_one(&state.db_pool)
    .await?;

    Ok(Json(rec.id))
}

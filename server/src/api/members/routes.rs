use std::sync::Arc;

use axum::{
    extract::{Multipart, Path, Query, State},
    response::IntoResponse,
    Json,
};
use chrono::{NaiveDate, NaiveTime, Utc};
use indexmap::IndexMap;
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};
use uuid::Uuid;

use crate::{api::users::models::UserRole, auth::AuthExtractor, Gender, InnerAppState};

use super::{
    models::{
        CreateMemberBuilder, MemberResponse, MemberResponseBrief, MemberRow, MemberRowWithParents,
        RequestStatus, RequestedMemberResponseBrief, RequestedMemberRow,
        RequestedMemberRowWithParents, UpdateMemberBuilder,
    },
    MembersError,
};

const FIELDS_LIMIT: i32 = 10;

/// Get family members
#[axum::debug_handler]
pub async fn get_members(
    State(state): State<Arc<InnerAppState>>,
) -> anyhow::Result<Json<MemberResponse>, MembersError> {
    let recs: Vec<MemberRowWithParents> = sqlx::query_as(
        r#"
SELECT
    m.id,
    m.name,
    m.gender,
    m.birthday,
    m.last_name,
    m.image,
    m.image_type,
    m.personal_info,
    mother.id AS mother_id,
    mother.name AS mother_name,
    mother.gender AS mother_gender,
    mother.birthday AS mother_birthday,
    mother.last_name AS mother_last_name,
    father.id AS father_id,
    father.name AS father_name,
    father.gender AS father_gender,
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
        return Err(MembersError::NoMembers);
    }

    let Some(root) = recs
        .iter()
        .find(|rec| rec.father_id.is_none() && rec.mother_id.is_none())
    else {
        return Err(MembersError::NoRootMember);
    };

    let mut root = MemberResponse {
        id: root.id,
        name: root.name.clone(),
        gender: root.gender,
        birthday: root.birthday,
        last_name: root.last_name.clone(),
        father_id: None,
        mother_id: None,
        personal_info: root.personal_info.as_ref().and_then(|p| {
            p.as_object().map(|o| {
                o.into_iter()
                    .map(|(k, v)| (k.to_string(), v.as_str().unwrap_or("").to_string()))
                    .rev()
                    .collect::<IndexMap<String, String>>()
            })
        }),
        children: Vec::new(),
        image: root.image.clone(),
        image_type: root.image_type.clone(),
    };

    root.add_all_children(&recs);

    Ok(Json(root))
}

#[serde_as]
#[derive(Clone, Deserialize)]
pub struct FlatMembersParams {
    pub query: Option<String>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub page: Option<usize>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub per_page: Option<usize>,
}

/// Get family members as a flat vector
#[axum::debug_handler]
pub async fn get_members_flat(
    State(state): State<Arc<InnerAppState>>,
    Query(params): Query<FlatMembersParams>,
) -> anyhow::Result<Json<Vec<MemberResponseBrief>>, MembersError> {
    let per_page = params.per_page.unwrap_or(10);

    let recs: Vec<MemberRowWithParents> = if let Some(search_term) = params.query {
        sqlx::query_as(
            r#"
        SELECT
            m.id,
            m.name,
            m.gender,
            m.birthday,
            m.last_name,
            m.image,
            m.image_type,
            m.personal_info,
            mother.id as mother_id,
            mother.name AS mother_name,
            mother.gender AS mother_gender,
            mother.birthday AS mother_birthday,
            mother.last_name AS mother_last_name,
            father.id as father_id,
            father.name AS father_name,
            father.gender AS father_gender,
            father.birthday AS father_birthday,
            father.last_name AS father_last_name
        FROM
            members m
        LEFT JOIN
            members mother ON m.mother_id = mother.id
        LEFT JOIN
            members father ON m.father_id = father.id
        WHERE
        to_tsvector(m.name) @@ websearch_to_tsquery($1)
        ORDER BY
            m.id, m.name ASC
        OFFSET $2
        LIMIT $3;
            "#,
        )
        .bind(search_term)
        .bind((params.page.unwrap_or(0) * per_page).saturating_sub(1) as i32)
        .bind(per_page as i32)
        .fetch_all(&state.db_pool)
        .await?
    } else {
        sqlx::query_as(
            r#"
        SELECT
            m.id,
            m.name,
            m.gender,
            m.birthday,
            m.last_name,
            m.image,
            m.image_type,
            m.personal_info,
            mother.id as mother_id,
            mother.name AS mother_name,
            mother.gender AS mother_gender,
            mother.birthday AS mother_birthday,
            mother.last_name AS mother_last_name,
            father.id as father_id,
            father.name AS father_name,
            father.gender AS father_gender,
            father.birthday AS father_birthday,
            father.last_name AS father_last_name
        FROM
            members m
        LEFT JOIN
            members mother ON m.mother_id = mother.id
        LEFT JOIN
            members father ON m.father_id = father.id
        ORDER BY
            m.id, m.name ASC
        OFFSET $1
        LIMIT $2;
            "#,
        )
        .bind((params.page.unwrap_or(0) * per_page).saturating_sub(1) as i32)
        .bind(per_page as i32)
        .fetch_all(&state.db_pool)
        .await?
    };

    if recs.is_empty() {
        return Err(MembersError::NoMembers);
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
            personal_info: r.personal_info.and_then(|p| {
                p.as_object().map(|o| {
                    o.into_iter()
                        .map(|(k, v)| (k.to_string(), v.as_str().unwrap_or("").to_string()))
                        .rev()
                        .collect::<IndexMap<String, String>>()
                })
            }),
            image: r.image,
            image_type: r.image_type,
        })
        .collect();

    Ok(Json(members))
}

/// Add a family member
pub async fn add_member(
    _auth: AuthExtractor<{ UserRole::Admin as u8 }>,
    State(state): State<Arc<InnerAppState>>,
    mut multipart: Multipart,
) -> anyhow::Result<(), MembersError> {
    let mut limit = FIELDS_LIMIT;
    let mut create_member_builder = CreateMemberBuilder::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_e| MembersError::SomethingWentWrong)?
    {
        match field.name() {
            Some("name") => {
                let Ok(name) = field.text().await else {
                    return Err(MembersError::InvalidValue(String::from("name")));
                };

                if name.is_empty() {
                    return Err(MembersError::InvalidValue(String::from("name")));
                }

                create_member_builder.name(name);
            }
            Some("last_name") => {
                let Ok(last_name) = field.text().await else {
                    return Err(MembersError::InvalidValue(String::from("last_name")));
                };

                if last_name.is_empty() {
                    return Err(MembersError::InvalidValue(String::from("last_name")));
                }

                create_member_builder.last_name(last_name);
            }
            Some("gender") => {
                let Ok(gender) = field.text().await else {
                    return Err(MembersError::InvalidValue(String::from("gender")));
                };
                if gender.to_lowercase() == "male" {
                    create_member_builder.gender(Gender::Male);
                } else if gender.to_lowercase() == "female" {
                    create_member_builder.gender(Gender::Female);
                } else {
                    return Err(MembersError::InvalidValue(String::from("gender")));
                }
            }
            Some("birthday") => {
                let Ok(birthday) = field.text().await else {
                    return Err(MembersError::InvalidValue(String::from("birthday")));
                };
                let birthday = NaiveDate::parse_from_str(&birthday, "%Y-%m-%d")
                    .map_err(|e| {
                        log::error!("birthday error: {e}");
                        MembersError::InvalidValue(String::from("birthday"))
                    })?
                    .and_time(
                        NaiveTime::from_hms_opt(0, 0, 1).expect("00:00:01 should be a valid time"),
                    )
                    .and_utc();
                create_member_builder.birthday(birthday);
            }
            Some("father_id") => {
                let Ok(father_id) = field.text().await else {
                    return Err(MembersError::InvalidValue(String::from("father_id")));
                };

                if father_id.is_empty() {
                    continue;
                }

                create_member_builder.father_id(
                    father_id
                        .parse()
                        .map_err(|_e| MembersError::InvalidValue(String::from("father_id")))?,
                );
            }
            Some("mother_id") => {
                let Ok(mother_id) = field.text().await else {
                    return Err(MembersError::InvalidValue(String::from("mother_id")));
                };

                if mother_id.is_empty() {
                    continue;
                }

                create_member_builder.mother_id(
                    mother_id
                        .parse()
                        .map_err(|_e| MembersError::InvalidValue(String::from("mother_id")))?,
                );
            }
            Some("image") => {
                if let Some(image_content_type) = field.content_type() {
                    let image_content_type = image_content_type.to_string();
                    match image_content_type.as_str() {
                        "image/png" | "image/jpg" | "image/jpeg" => {
                            let Ok(image) = field.bytes().await else {
                                return Err(MembersError::InvalidValue(String::from("image")));
                            };

                            if image.is_empty() {
                                continue;
                            }

                            create_member_builder.image(image.to_vec());
                            create_member_builder.image_type(image_content_type.to_string());
                        }
                        mime_type => {
                            log::debug!("{mime_type}");
                            return Err(MembersError::InvalidImage);
                        }
                    }
                } else {
                    continue;
                }
            }
            Some("info") => {
                let Ok(info) = field.text().await else {
                    return Err(MembersError::InvalidValue(String::from("info")));
                };

                if info.is_empty() {
                    continue;
                }

                create_member_builder.info(
                    serde_json::from_str(&info)
                        .map_err(|_e| MembersError::InvalidValue(String::from("info")))?,
                );
            }
            Some(field) => return Err(MembersError::InvalidField(field.to_string())),
            None => {
                return Err(MembersError::BadRequest);
            }
        }
        if limit > 0 {
            limit -= 1;
        } else {
            break;
        }
    }

    let create_member = create_member_builder.build()?;
    let info = create_member.info.and_then(|info| {
        sqlx::types::JsonValue::deserialize(serde::de::value::MapDeserializer::new(
            info.into_iter(),
        ))
        .ok()
    });

    sqlx::query(
        r#"
    INSERT INTO members (name, gender, birthday, last_name, father_id, mother_id, image, image_type, personal_info)
    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
    RETURNING id
            "#,
    )
    .bind(create_member.name)
    .bind(create_member.gender)
    .bind(create_member.birthday)
    .bind(create_member.last_name)
    .bind(create_member.father_id)
    .bind(create_member.mother_id)
    .bind(create_member.image)
    .bind(create_member.image_type)
    .bind(info)
    .execute(&state.db_pool)
    .await?;

    Ok(())
}

/// Edit a family member
pub async fn edit_member(
    _auth: AuthExtractor<{ UserRole::Admin as u8 }>,
    State(state): State<Arc<InnerAppState>>,
    Path(id): Path<i64>,
    mut multipart: Multipart,
) -> anyhow::Result<(), MembersError> {
    let mut limit = FIELDS_LIMIT;
    let mut update_member_builder = UpdateMemberBuilder::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_e| MembersError::SomethingWentWrong)?
    {
        match field.name() {
            Some("name") => {
                let Ok(name) = field.text().await else {
                    return Err(MembersError::InvalidValue(String::from("name")));
                };
                update_member_builder.name(name);
            }
            Some("last_name") => {
                let Ok(last_name) = field.text().await else {
                    return Err(MembersError::InvalidValue(String::from("last_name")));
                };
                update_member_builder.last_name(last_name);
            }
            Some("gender") => {
                let Ok(gender) = field.text().await else {
                    return Err(MembersError::InvalidValue(String::from("gender")));
                };
                if gender.to_lowercase() == "male" {
                    update_member_builder.gender(Gender::Male);
                } else if gender.to_lowercase() == "female" {
                    update_member_builder.gender(Gender::Female);
                } else {
                    return Err(MembersError::InvalidValue(String::from("gender")));
                }
            }
            Some("birthday") => {
                let Ok(birthday) = field.text().await else {
                    return Err(MembersError::InvalidValue(String::from("birthday")));
                };
                let birthday = NaiveDate::parse_from_str(&birthday, "%Y-%m-%d")
                    .map_err(|e| {
                        log::error!("birthday error: {e}");
                        MembersError::InvalidValue(String::from("birthday"))
                    })?
                    .and_time(
                        NaiveTime::from_hms_opt(0, 0, 1).expect("00:00:01 should be a valid time"),
                    )
                    .and_utc();
                update_member_builder.birthday(birthday);
            }
            Some("father_id") => {
                let Ok(father_id) = field.text().await else {
                    return Err(MembersError::InvalidValue(String::from("father_id")));
                };

                if father_id.is_empty() {
                    update_member_builder.remove_father_id(true);
                    continue;
                }

                update_member_builder.father_id(
                    father_id
                        .parse()
                        .map_err(|_e| MembersError::InvalidValue(String::from("father_id")))?,
                );
            }
            Some("mother_id") => {
                let Ok(mother_id) = field.text().await else {
                    return Err(MembersError::InvalidValue(String::from("mother_id")));
                };

                if mother_id.is_empty() {
                    update_member_builder.remove_mother_id(true);
                    continue;
                }

                update_member_builder.mother_id(
                    mother_id
                        .parse()
                        .map_err(|_e| MembersError::InvalidValue(String::from("mother_id")))?,
                );
            }
            Some("image") => {
                if let Some(image_content_type) = field.content_type() {
                    let image_content_type = image_content_type.to_string();
                    match image_content_type.as_str() {
                        "image/png" | "image/jpg" | "image/jpeg" => {
                            let Ok(image) = field.bytes().await else {
                                return Err(MembersError::InvalidValue(String::from("image")));
                            };

                            // TODO: support removing member image
                            // if image.is_empty() {

                            // }

                            update_member_builder.image(image.to_vec());
                            update_member_builder.image_type(image_content_type);
                        }
                        _ => {
                            return Err(MembersError::InvalidImage);
                        }
                    }
                } else {
                    continue;
                }
            }
            Some("info") => {
                let Ok(info) = field.text().await else {
                    return Err(MembersError::InvalidValue(String::from("info")));
                };

                if info.is_empty() {
                    update_member_builder.remove_info(true);
                    continue;
                }

                update_member_builder.info(
                    serde_json::from_str(&info)
                        .map_err(|_e| MembersError::InvalidValue(String::from("info")))?,
                );
            }
            Some(field) => {
                return Err(MembersError::InvalidField(field.to_string()));
            }
            None => {
                return Err(MembersError::BadRequest);
            }
        }
        if limit > 0 {
            limit -= 1;
        } else {
            break;
        }
    }

    let remove_father_id = update_member_builder.remove_father_id;
    let remove_mother_id = update_member_builder.remove_mother_id;
    let remove_info = update_member_builder.remove_info;
    let update_member = update_member_builder.build(id)?;

    let mut tx = state.db_pool.begin().await?;

    if let Some(name) = &update_member.name {
        sqlx::query(
            r#"
    UPDATE members
    SET name = $2
    WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(name)
        .execute(&mut *tx)
        .await?;
    }

    if let Some(last_name) = &update_member.last_name {
        log::debug!("id: {}", update_member.id);
        log::debug!("last_name: {}", last_name);

        sqlx::query(
            r#"
UPDATE members
SET last_name = $1::TEXT
WHERE id = $2::INTEGER"#,
        )
        .bind(last_name)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    }

    if let Some(birthday) = &update_member.birthday {
        sqlx::query(
            r#"
    UPDATE members
    SET birthday = $2
    WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(birthday)
        .execute(&mut *tx)
        .await?;
    }

    if let Some(gender) = &update_member.gender {
        sqlx::query(
            r#"
UPDATE members
SET gender = $2
WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(gender)
        .execute(&mut *tx)
        .await?;
    }

    if let Some(mother_id) = &update_member.mother_id {
        sqlx::query(
            r#"
    UPDATE members
    SET mother_id = $2
    WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(mother_id)
        .execute(&mut *tx)
        .await?;
    } else if remove_mother_id {
        sqlx::query(
            r#"
    UPDATE members
    SET mother_id = NULL
    WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;
    }

    if let Some(father_id) = &update_member.father_id {
        sqlx::query(
            r#"
    UPDATE members
    SET father_id = $2
    WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(father_id)
        .execute(&mut *tx)
        .await?;
    } else if remove_father_id {
        sqlx::query(
            r#"
    UPDATE members
    SET father_id = NULL
    WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;
    }

    if let Some(info) = &update_member.info {
        let info = sqlx::types::JsonValue::deserialize(serde::de::value::MapDeserializer::new(
            info.clone().into_iter(),
        ))
        .map_err(|_e| MembersError::SomethingWentWrong)?;

        sqlx::query(
            r#"
    UPDATE members
    SET personal_info = $2
    WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(info)
        .execute(&mut *tx)
        .await?;
    } else if remove_info {
        sqlx::query(
            r#"
    UPDATE members
    SET personal_info = NULL
    WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;
    }

    if let Some((image, image_type)) = &update_member
        .image
        .and_then(|i| update_member.image_type.map(|it| (i, it)))
    {
        sqlx::query(
            r#"
UPDATE members
SET image = $2, image_type = $3
WHERE id = $1"#,
        )
        .bind(update_member.id)
        .bind(image)
        .bind(image_type)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    Ok(())
}

/// Remove a family member
pub async fn delete_member(
    _auth: AuthExtractor<{ UserRole::Admin as u8 }>,
    State(state): State<Arc<InnerAppState>>,
    Path(id): Path<i64>,
) -> anyhow::Result<(), MembersError> {
    sqlx::query(
        r#"
DELETE FROM members WHERE id = $1"#,
    )
    .bind(id)
    .execute(&state.db_pool)
    .await?;

    Ok(())
}

pub async fn export_members(
    _auth: AuthExtractor<{ UserRole::Admin as u8 }>,
    State(state): State<Arc<InnerAppState>>,
) -> Result<impl IntoResponse, MembersError> {
    let recs: Vec<MemberRow> = sqlx::query_as(
        r#"
SELECT
m.id,
m.name,
m.gender,
m.birthday,
m.last_name,
m.image,
m.image_type,
m.personal_info,
m.father_id,
m.mother_id
FROM members m
"#,
    )
    .fetch_all(&state.db_pool)
    .await?;

    let mut csv_writer = csv::Writer::from_writer(vec![]);

    for rec in recs {
        csv_writer.serialize(rec).map_err(|e| {
            log::error!("{e}");
            MembersError::SomethingWentWrong
        })?;
    }

    let headers = [
        (axum::http::header::CONTENT_TYPE, "text/csv"),
        (
            axum::http::header::CONTENT_DISPOSITION,
            &r#"attachment; filename="exported-members.csv""#,
        ),
    ];

    csv_writer.flush().map_err(|e| {
        log::error!("{e}");
        MembersError::SomethingWentWrong
    })?;

    let data = csv_writer.into_inner().map_err(|e| {
        log::error!("{e}");
        MembersError::SomethingWentWrong
    })?;

    Ok((headers, data))
}

pub async fn upload_members_csv(
    _auth: AuthExtractor<{ UserRole::Admin as u8 }>,
    State(state): State<Arc<InnerAppState>>,
    mut multipart: Multipart,
) -> Result<(), MembersError> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_e| MembersError::SomethingWentWrong)?
    {
        match field.name() {
            Some("members_csv") => {
                let file_data = field.text().await.map_err(|e| {
                    log::error!("{e}");
                    MembersError::SomethingWentWrong
                })?;

                let mut csv_reader = csv::ReaderBuilder::new()
                    .delimiter(b',')
                    .from_reader(file_data.as_bytes());

                let members: Vec<MemberRow> = csv_reader
                    .deserialize::<MemberRow>()
                    .map(|r| {
                        r.map_err(|e| {
                            log::error!("{e}");
                            MembersError::SomethingWentWrong
                        })
                    })
                    .collect::<Result<Vec<MemberRow>, MembersError>>()?;

                let mut tx = state.db_pool.begin().await?;

                let mut query = sqlx::QueryBuilder::new("INSERT INTO members (id, name, last_name, gender, birthday, mother_id, father_id)");

                query.push_values(members, |mut b, members| {
                    b.push_bind(members.id)
                        .push_bind(members.name)
                        .push_bind(members.last_name)
                        .push_bind(members.gender)
                        .push_bind(members.birthday)
                        .push_bind(members.mother_id)
                        .push_bind(members.father_id);
                });

                query.push(r#"
                    ON CONFLICT(id)
                    DO UPDATE SET
                    name = EXCLUDED.name, last_name = EXCLUDED.last_name, gender = EXCLUDED.gender,
                    birthday = EXCLUDED.birthday, mother_id = EXCLUDED.mother_id, father_id = EXCLUDED.father_id
                "#);

                query.build().execute(&mut *tx).await?;

                sqlx::query(r#"SELECT setval('members_id_seq', (SELECT MAX(id) FROM members));"#)
                    .execute(&mut *tx)
                    .await?;

                tx.commit().await?;
            }
            Some(_) => {
                continue;
            }
            None => {
                return Err(MembersError::BadRequest);
            }
        }
    }

    Ok(())
}

/// Request adding a family member
pub async fn request_add_member(
    State(state): State<Arc<InnerAppState>>,
    mut multipart: Multipart,
) -> anyhow::Result<(), MembersError> {
    let mut limit = FIELDS_LIMIT;
    let mut new_member_builder = CreateMemberBuilder::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_e| MembersError::SomethingWentWrong)?
    {
        match field.name() {
            Some("name") => {
                let Ok(name) = field.text().await else {
                    return Err(MembersError::InvalidValue(String::from("name")));
                };

                if name.is_empty() {
                    return Err(MembersError::InvalidValue(String::from("name")));
                }

                new_member_builder.name(name);
            }
            Some("last_name") => {
                let Ok(last_name) = field.text().await else {
                    return Err(MembersError::InvalidValue(String::from("last_name")));
                };

                if last_name.is_empty() {
                    return Err(MembersError::InvalidValue(String::from("last_name")));
                }

                new_member_builder.last_name(last_name);
            }
            Some("gender") => {
                let Ok(gender) = field.text().await else {
                    return Err(MembersError::InvalidValue(String::from("gender")));
                };
                if gender.to_lowercase() == "male" {
                    new_member_builder.gender(Gender::Male);
                } else if gender.to_lowercase() == "female" {
                    new_member_builder.gender(Gender::Female);
                } else {
                    return Err(MembersError::InvalidValue(String::from("gender")));
                }
            }
            Some("birthday") => {
                let Ok(birthday) = field.text().await else {
                    return Err(MembersError::InvalidValue(String::from("birthday")));
                };
                let birthday = NaiveDate::parse_from_str(&birthday, "%Y-%m-%d")
                    .map_err(|e| {
                        log::error!("birthday error: {e}");
                        MembersError::InvalidValue(String::from("birthday"))
                    })?
                    .and_time(
                        NaiveTime::from_hms_opt(0, 0, 1).expect("00:00:01 should be a valid time"),
                    )
                    .and_utc();
                new_member_builder.birthday(birthday);
            }
            Some("father_id") => {
                let Ok(father_id) = field.text().await else {
                    return Err(MembersError::InvalidValue(String::from("father_id")));
                };

                if father_id.is_empty() {
                    return Err(MembersError::InvalidValue(String::from("father_id")));
                }

                new_member_builder.father_id(
                    father_id
                        .parse()
                        .map_err(|_e| MembersError::InvalidValue(String::from("father_id")))?,
                );
            }
            Some("mother_id") => {
                let Ok(mother_id) = field.text().await else {
                    return Err(MembersError::InvalidValue(String::from("mother_id")));
                };

                if mother_id.is_empty() {
                    continue;
                }

                new_member_builder.mother_id(
                    mother_id
                        .parse()
                        .map_err(|_e| MembersError::InvalidValue(String::from("mother_id")))?,
                );
            }
            Some("image") => {
                if let Some(image_content_type) = field.content_type() {
                    let image_content_type = image_content_type.to_string();
                    match image_content_type.as_str() {
                        "image/png" | "image/jpg" | "image/jpeg" => {
                            let Ok(image) = field.bytes().await else {
                                return Err(MembersError::InvalidValue(String::from("image")));
                            };

                            if image.is_empty() {
                                continue;
                            }

                            new_member_builder.image(image.to_vec());
                            new_member_builder.image_type(image_content_type.to_string());
                        }
                        mime_type => {
                            log::debug!("{mime_type}");
                            return Err(MembersError::InvalidImage);
                        }
                    }
                } else {
                    continue;
                }
            }
            Some("info") => {
                let Ok(info) = field.text().await else {
                    return Err(MembersError::InvalidValue(String::from("info")));
                };

                if info.is_empty() {
                    continue;
                }

                new_member_builder.info(
                    serde_json::from_str(&info)
                        .map_err(|_e| MembersError::InvalidValue(String::from("info")))?,
                );
            }
            Some(field) => return Err(MembersError::InvalidField(field.to_string())),
            None => {
                return Err(MembersError::BadRequest);
            }
        }
        if limit > 0 {
            limit -= 1;
        } else {
            break;
        }
    }

    let new_member = new_member_builder.build()?;
    let info = new_member.info.and_then(|info| {
        sqlx::types::JsonValue::deserialize(serde::de::value::MapDeserializer::new(
            info.into_iter(),
        ))
        .ok()
    });

    sqlx::query(
        r#"
    INSERT INTO member_add_requests (id, name, gender, birthday, last_name, father_id, mother_id, image, image_type, personal_info, submitted_at)
    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
    RETURNING id
            "#,
    )
    .bind(uuid::Uuid::new_v4())
    .bind(new_member.name)
    .bind(new_member.gender)
    .bind(new_member.birthday)
    .bind(new_member.last_name)
    .bind(new_member.father_id)
    .bind(new_member.mother_id)
    .bind(new_member.image)
    .bind(new_member.image_type)
    .bind(info)
    .bind(Utc::now())
    .execute(&state.db_pool)
    .await?;

    Ok(())
}

/// Get requested members as a flat vector
#[axum::debug_handler]
pub async fn get_requested_members_flat(
    State(state): State<Arc<InnerAppState>>,
    Query(params): Query<FlatMembersParams>,
) -> anyhow::Result<Json<Vec<RequestedMemberResponseBrief>>, MembersError> {
    let per_page = params.per_page.unwrap_or(10);

    let recs: Vec<RequestedMemberRowWithParents> = if let Some(search_term) = params.query {
        sqlx::query_as(
            r#"
        SELECT
            m.id,
            m.name,
            m.gender,
            m.birthday,
            m.last_name,
            m.image,
            m.image_type,
            m.personal_info,
            m.status,
            mother.id as mother_id,
            mother.name AS mother_name,
            mother.gender AS mother_gender,
            mother.birthday AS mother_birthday,
            mother.last_name AS mother_last_name,
            father.id as father_id,
            father.name AS father_name,
            father.gender AS father_gender,
            father.birthday AS father_birthday,
            father.last_name AS father_last_name
        FROM
            member_add_requests m
        LEFT JOIN
            members mother ON m.mother_id = mother.id
        LEFT JOIN
            members father ON m.father_id = father.id
        WHERE
        to_tsvector(m.name) @@ websearch_to_tsquery($1)
        ORDER BY
            m.submitted_at, m.name ASC
        OFFSET $2
        LIMIT $3;
            "#,
        )
        .bind(search_term)
        .bind((params.page.unwrap_or(0) * per_page).saturating_sub(1) as i32)
        .bind(per_page as i32)
        .fetch_all(&state.db_pool)
        .await?
    } else {
        sqlx::query_as(
            r#"
        SELECT
            m.id,
            m.name,
            m.gender,
            m.birthday,
            m.last_name,
            m.image,
            m.image_type,
            m.personal_info,
            m.status,
            mother.id as mother_id,
            mother.name AS mother_name,
            mother.gender AS mother_gender,
            mother.birthday AS mother_birthday,
            mother.last_name AS mother_last_name,
            father.id as father_id,
            father.name AS father_name,
            father.gender AS father_gender,
            father.birthday AS father_birthday,
            father.last_name AS father_last_name
        FROM
            member_add_requests m
        LEFT JOIN
            members mother ON m.mother_id = mother.id
        LEFT JOIN
            members father ON m.father_id = father.id
        ORDER BY
            m.submitted_at, m.name ASC
        OFFSET $1
        LIMIT $2;
            "#,
        )
        .bind((params.page.unwrap_or(0) * per_page).saturating_sub(1) as i32)
        .bind(per_page as i32)
        .fetch_all(&state.db_pool)
        .await?
    };

    let members = recs
        .into_iter()
        .map(|r| RequestedMemberResponseBrief {
            id: r.id,
            name: r.name,
            gender: r.gender,
            birthday: r.birthday,
            last_name: r.last_name,
            father_id: r.father_id,
            mother_id: r.mother_id,
            personal_info: r.personal_info.and_then(|p| {
                p.as_object().map(|o| {
                    o.into_iter()
                        .map(|(k, v)| (k.to_string(), v.as_str().unwrap_or("").to_string()))
                        .rev()
                        .collect::<IndexMap<String, String>>()
                })
            }),
            image: r.image,
            image_type: r.image_type,
            status: r.status,
        })
        .collect();

    Ok(Json(members))
}

/// Approve a member add request
pub async fn approve_member_request(
    _auth: AuthExtractor<{ UserRole::Admin as u8 }>,
    State(state): State<Arc<InnerAppState>>,
    Path(id): Path<Uuid>,
) -> anyhow::Result<(), MembersError> {
    let mut tx = state.db_pool.begin().await?;

    let member: RequestedMemberRow = sqlx::query_as(
        r#"
UPDATE member_add_requests
SET status = $1
WHERE id = $2 AND status = $3
RETURNING *;
"#,
    )
    .bind(RequestStatus::Approved)
    .bind(id)
    .bind(RequestStatus::Pending)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        r#"
    INSERT INTO members (name, gender, birthday, last_name, father_id, mother_id, image, image_type, personal_info)
    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
    RETURNING id
            "#,
    )
    .bind(member.name)
    .bind(member.gender)
    .bind(member.birthday)
    .bind(member.last_name)
    .bind(member.father_id)
    .bind(member.mother_id)
    .bind(member.image)
    .bind(member.image_type)
    .bind(member.personal_info)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(())
}

/// Dispprove a member add request
pub async fn disapprove_member_request(
    _auth: AuthExtractor<{ UserRole::Admin as u8 }>,
    State(state): State<Arc<InnerAppState>>,
    Path(id): Path<Uuid>,
) -> anyhow::Result<(), MembersError> {
    let lmao = sqlx::query(
        r#"
UPDATE member_add_requests
SET status = $1
WHERE id = $2 AND status = $3;
"#,
    )
    .bind(RequestStatus::Disapproved)
    .bind(id)
    .bind(RequestStatus::Pending)
    .execute(&state.db_pool)
    .await?;

    if lmao.rows_affected() < 1 {
        return Err(MembersError::BadRequest);
    }

    Ok(())
}

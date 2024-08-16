use std::sync::Arc;

use axum::{
    extract::{Multipart, Path, State},
    response::IntoResponse,
    Json,
};
use chrono::{NaiveDate, NaiveTime};
use indexmap::IndexMap;
use serde::Deserialize;

use crate::{
    api::{members::MemberRow, users::models::UserRole},
    auth::AuthExtractor,
    Gender, InnerAppState,
};

use super::{
    CreateMemberBuilder, MemberResponse, MemberResponseBrief, MemberRowWithParents, MembersError,
    UpdateMemberBuilder,
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

/// Get family members as a flat vector
#[axum::debug_handler]
pub async fn get_members_flat(
    State(state): State<Arc<InnerAppState>>,
) -> anyhow::Result<Json<Vec<MemberResponseBrief>>, MembersError> {
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
    members father ON m.father_id = father.id;
    "#,
    )
    .fetch_all(&state.db_pool)
    .await?;

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

    while let Some(field) = multipart.next_field().await.unwrap() {
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
    Path(id): Path<i32>,
    mut multipart: Multipart,
) -> anyhow::Result<(), MembersError> {
    let mut limit = FIELDS_LIMIT;
    let mut update_member_builder = UpdateMemberBuilder::new();

    while let Some(field) = multipart.next_field().await.unwrap() {
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
        .unwrap();

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
    Path(id): Path<i32>,
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
    while let Some(field) = multipart.next_field().await.unwrap() {
        match field.name() {
            Some("members_csv") => {
                let file_data = field.bytes().await.map_err(|e| {
                    log::error!("{e}");
                    MembersError::SomethingWentWrong
                })?;

                let file_data = file_data.to_vec();

                let mut csv_reader = csv::Reader::from_reader(file_data.as_slice());

                let members: Vec<MemberRow> = csv_reader
                    .deserialize::<MemberRow>()
                    .map(|r| {
                        r.map_err(|e| {
                            log::error!("{e}");
                            MembersError::SomethingWentWrong
                        })
                    })
                    .collect::<Result<Vec<MemberRow>, MembersError>>()?;

                for member in members {
                    let query = sqlx::query(
                        r#"
                                UPDATE members
                                SET name = $1, last_name = $2, gender = $3, birthday = $4, mother_id = $5, father_id = $6
                                WHERE id = $7
                                RETURNING id
                            "#,
                    )
                    .bind(&member.name)
                    .bind(&member.last_name)
                    .bind(member.gender)
                    .bind(member.birthday)
                    .bind(member.mother_id)
                    .bind(member.father_id)
                    .bind(member.id)
                    .fetch_optional(&state.db_pool).await?;

                    if query.is_none() {
                        sqlx::query(
                            r#"
                                INSERT INTO members (name, last_name, gender, birthday, mother_id, father_id)
                                VALUES ($1, $2, $3, $4, $5, $6)
                                "#,
                        )
                        .bind(&member.name)
                        .bind(&member.last_name)
                        .bind(member.gender)
                        .bind(member.birthday)
                        .bind(member.mother_id)
                        .bind(member.father_id)
                        .execute(&state.db_pool).await?;
                    }
                }
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

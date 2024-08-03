use std::sync::Arc;

use axum::{
    extract::{Multipart, Path, State},
    Json,
};
use chrono::{DateTime, NaiveDate, NaiveTime};

use crate::{api::users::models::UserRole, auth::AuthExtractor, Gender, InnerAppState};

use super::{
    CreateMemberBuilder, MemberResponse, MemberResponseBrief, MemberRow, MembersError,
    UpdateMemberBuilder,
};

const FIELDS_LIMIT: i32 = 10;

/// Get family members
#[axum::debug_handler]
pub async fn get_members(
    State(state): State<Arc<InnerAppState>>,
) -> anyhow::Result<Json<MemberResponse>, MembersError> {
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
        children: Vec::new(),
    };

    root.add_all_children(&recs);

    Ok(Json(root))
}

/// Get family members as a flat vector
#[axum::debug_handler]
pub async fn get_members_flat(
    State(state): State<Arc<InnerAppState>>,
) -> anyhow::Result<Json<Vec<MemberResponseBrief>>, MembersError> {
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
        })
        .collect();

    Ok(Json(members))
}

/// Add a family member
pub async fn add_member(
    _auth: AuthExtractor<{ UserRole::Admin as u8 }>,
    State(state): State<Arc<InnerAppState>>,
    mut multipart: Multipart,
) -> anyhow::Result<Json<i32>, MembersError> {
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
                log::debug!("birthday value: {birthday:?}");
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
                    match image_content_type {
                        "image/png" | "image/jpg" | "image/jpeg" => {
                            let Ok(image) = field.bytes().await else {
                                return Err(MembersError::InvalidValue(String::from("image")));
                            };
                            if image.is_empty() {
                                return Err(MembersError::InvalidValue(String::from("image")));
                            }

                            create_member_builder.image(image.to_vec());
                        }
                        _ => {
                            return Err(MembersError::InvalidImage);
                        }
                    }
                } else {
                    return Err(MembersError::InvalidImage);
                }
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
                let birthday = DateTime::parse_from_rfc2822(&birthday)
                    .or(DateTime::parse_from_rfc3339(&birthday))
                    .map_err(|_e| MembersError::InvalidValue(String::from("birthday")))?;
                update_member_builder.birthday(birthday.to_utc());
            }
            Some("father_id") => {
                let Ok(father_id) = field.text().await else {
                    return Err(MembersError::InvalidValue(String::from("father_id")));
                };
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
                update_member_builder.mother_id(
                    mother_id
                        .parse()
                        .map_err(|_e| MembersError::InvalidValue(String::from("mother_id")))?,
                );
            }
            Some("image") => {
                if let Some(image_content_type) = field.content_type() {
                    match image_content_type {
                        "image/png" | "image/jpg" | "image/jpeg" => {
                            let Ok(image) = field.bytes().await else {
                                return Err(MembersError::InvalidValue(String::from("image")));
                            };
                            update_member_builder.image(image.to_vec());
                        }
                        _ => {
                            return Err(MembersError::InvalidImage);
                        }
                    }
                } else {
                    return Err(MembersError::InvalidImage);
                }
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

    let update_member = update_member_builder.build(id)?;

    let mut tx = state.db_pool.begin().await?;

    if let Some(name) = &update_member.name {
        sqlx::query!(
            r#"
    UPDATE members
    SET name = $2
    WHERE id = $1
            "#,
            id,
            name,
        )
        .execute(&mut *tx)
        .await?;
    }

    if let Some(last_name) = &update_member.last_name {
        log::debug!("id: {}", update_member.id);
        log::debug!("last_name: {}", last_name);

        sqlx::query!(
            r#"
UPDATE members
SET last_name = $1::TEXT
WHERE id = $2::INTEGER"#,
            last_name,
            id,
        )
        .execute(&mut *tx)
        .await?;
    }

    if let Some(birthday) = &update_member.birthday {
        sqlx::query!(
            r#"
    UPDATE members
    SET birthday = $2
    WHERE id = $1
            "#,
            id,
            birthday,
        )
        .execute(&mut *tx)
        .await?;
    }

    if let Some(gender) = &update_member.gender {
        sqlx::query!(
            r#"
    UPDATE members
    SET gender = $2
    WHERE id = $1
    RETURNING gender as "gender: Gender"
            "#,
            id,
            gender as _,
        )
        .fetch_one(&mut *tx)
        .await?;
    }

    if let Some(mother_id) = &update_member.mother_id {
        sqlx::query!(
            r#"
    UPDATE members
    SET mother_id = $2
    WHERE id = $1
            "#,
            id,
            mother_id,
        )
        .execute(&mut *tx)
        .await?;
    }

    if let Some(father_id) = &update_member.father_id {
        sqlx::query!(
            r#"
    UPDATE members
    SET father_id = $2
    WHERE id = $1
            "#,
            id,
            father_id,
        )
        .execute(&mut *tx)
        .await?;
    }

    // TODO: image
    // if let Some(image) = &update_member.image {
    //     sqlx::query!(
    //         r#"
    // UPDATE members
    // SET image = $2
    // WHERE id = $1
    //         "#,
    //         update_member.id,
    //         image,
    //     )
    //     .execute(&mut *tx)
    //     .await?;
    // }

    tx.commit().await?;

    Ok(())
}

/// Remove a family member
pub async fn delete_member(
    _auth: AuthExtractor<{ UserRole::Admin as u8 }>,
    State(state): State<Arc<InnerAppState>>,
    Path(id): Path<i32>,
) -> anyhow::Result<(), MembersError> {
    sqlx::query!(
        r#"
DELETE FROM members WHERE id = $1"#,
        id,
    )
    .execute(&state.db_pool)
    .await?;

    Ok(())
}

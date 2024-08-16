use anyhow::anyhow;
use axum::{http::StatusCode, response::IntoResponse};
use chrono::{DateTime, Utc};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{auth::AuthError, ErrorResponse, Gender};

pub mod routes;

#[derive(thiserror::Error, Debug)]
pub enum MembersError {
    #[error("something went wrong")]
    SomethingWentWrong,

    #[error("bad request")]
    BadRequest,

    #[error("something went wrong")]
    Sqlx(#[from] sqlx::Error),

    #[error("no family members")]
    NoMembers,

    #[error("no root member")]
    NoRootMember,

    #[error("invalid {0} value")]
    InvalidValue(String),

    #[error("invalid field name: {0}")]
    InvalidField(String),

    #[error("invalid image type")]
    InvalidImage,

    #[error(transparent)]
    AuthError(#[from] AuthError),

    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}

impl IntoResponse for MembersError {
    fn into_response(self) -> axum::response::Response {
        log::error!("{:#?}", self);

        match self {
            MembersError::SomethingWentWrong => (StatusCode::INTERNAL_SERVER_ERROR).into_response(),
            MembersError::Sqlx(_) => (StatusCode::INTERNAL_SERVER_ERROR).into_response(),
            MembersError::AuthError(e) => e.into_response(),
            MembersError::NoMembers => (
                StatusCode::NOT_FOUND,
                ErrorResponse {
                    error: self.to_string(),
                    details: None,
                },
            )
                .into_response(),
            MembersError::NoRootMember => (
                StatusCode::NOT_FOUND,
                ErrorResponse {
                    error: self.to_string(),
                    details: None,
                },
            )
                .into_response(),
            MembersError::InvalidValue(_) => (
                StatusCode::BAD_REQUEST,
                ErrorResponse {
                    error: self.to_string(),
                    details: None,
                },
            )
                .into_response(),
            MembersError::InvalidImage => (
                StatusCode::BAD_REQUEST,
                ErrorResponse {
                    error: self.to_string(),
                    details: None,
                },
            )
                .into_response(),
            MembersError::InvalidField(_) => (
                StatusCode::BAD_REQUEST,
                ErrorResponse {
                    error: self.to_string(),
                    details: None,
                },
            )
                .into_response(),
            MembersError::BadRequest => (StatusCode::BAD_REQUEST).into_response(),
            MembersError::Anyhow(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorResponse {
                    error: self.to_string(),
                    details: None,
                },
            )
                .into_response(),
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct CreateMember {
    name: String,
    last_name: String,
    gender: Gender,
    birthday: chrono::DateTime<chrono::Utc>,
    mother_id: Option<i32>,
    father_id: Option<i32>,
    image: Option<Vec<u8>>,
    image_type: Option<String>,
    /// Generic info about family member
    /// a map is used to make it dynamic and hold any kind of personal information
    info: Option<IndexMap<String, serde_json::Value>>,
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
    image_type: Option<String>,
    info: Option<IndexMap<String, serde_json::Value>>,
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

    fn image_type(&mut self, image_type: String) -> &mut Self {
        self.image_type = Some(image_type);
        self
    }

    fn info(&mut self, info: IndexMap<String, serde_json::Value>) -> &mut Self {
        self.info = Some(info);
        self
    }

    fn build(self) -> anyhow::Result<CreateMember> {
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

        if self.image.is_some() != self.image_type.is_some() {
            return Err(anyhow!("image or image_type was not added"));
        }

        Ok(CreateMember {
            name,
            last_name,
            gender,
            birthday,
            mother_id: self.mother_id,
            father_id: self.father_id,
            image: self.image,
            info: self.info,
            image_type: self.image_type,
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateMember {
    id: i32,
    name: Option<String>,
    last_name: Option<String>,
    gender: Option<Gender>,
    birthday: Option<chrono::DateTime<chrono::Utc>>,
    mother_id: Option<i32>,
    father_id: Option<i32>,
    info: Option<IndexMap<String, serde_json::Value>>,
    image: Option<Vec<u8>>,
    image_type: Option<String>,
}

#[derive(Default)]
pub struct UpdateMemberBuilder {
    name: Option<String>,
    last_name: Option<String>,
    gender: Option<Gender>,
    birthday: Option<chrono::DateTime<chrono::Utc>>,
    mother_id: Option<i32>,
    remove_mother_id: bool,
    father_id: Option<i32>,
    remove_father_id: bool,
    info: Option<IndexMap<String, serde_json::Value>>,
    remove_info: bool,
    image: Option<Vec<u8>>,
    image_type: Option<String>,
}

impl UpdateMemberBuilder {
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

    fn remove_mother_id(&mut self, remove: bool) -> &mut Self {
        self.remove_mother_id = remove;
        self
    }

    fn father_id(&mut self, father_id: i32) -> &mut Self {
        self.father_id = Some(father_id);
        self
    }

    fn remove_father_id(&mut self, remove: bool) -> &mut Self {
        self.remove_father_id = remove;
        self
    }

    fn remove_info(&mut self, remove: bool) -> &mut Self {
        self.remove_info = remove;
        self
    }

    fn info(&mut self, info: IndexMap<String, serde_json::Value>) -> &mut Self {
        self.info = Some(info);
        self
    }

    fn image(&mut self, image: Vec<u8>) -> &mut Self {
        self.image = Some(image);
        self
    }

    fn image_type(&mut self, image_type: String) -> &mut Self {
        self.image_type = Some(image_type);
        self
    }

    fn build(self, id: i32) -> anyhow::Result<UpdateMember> {
        if self.image.is_some() != self.image_type.is_some() {
            return Err(anyhow!("image or image_type was not added"));
        }

        Ok(UpdateMember {
            id,
            name: self.name,
            last_name: self.last_name,
            gender: self.gender,
            birthday: self.birthday,
            mother_id: self.mother_id,
            father_id: self.father_id,
            image: self.image,
            image_type: self.image_type,
            info: self.info,
        })
    }
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
struct MemberRow {
    id: i32,
    name: String,
    last_name: String,
    gender: Gender,
    birthday: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip)]
    image: Option<Vec<u8>>,
    #[serde(skip)]
    image_type: Option<String>,
    #[serde(skip)]
    personal_info: Option<serde_json::Value>,
    mother_id: Option<i32>,
    father_id: Option<i32>,
}

#[allow(dead_code)]
#[derive(Debug, sqlx::FromRow)]
struct MemberRowWithParents {
    id: i32,
    name: String,
    gender: Gender,
    birthday: Option<chrono::DateTime<chrono::Utc>>,
    last_name: String,
    image: Option<Vec<u8>>,
    image_type: Option<String>,
    mother_id: Option<i32>,
    father_id: Option<i32>,
    personal_info: Option<serde_json::Value>,
    mother_name: Option<String>,
    mother_gender: Option<Gender>,
    mother_birthday: Option<chrono::DateTime<chrono::Utc>>,
    mother_last_name: Option<String>,
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
    pub personal_info: Option<IndexMap<String, String>>,
    children: Vec<MemberResponse>,
    image: Option<Vec<u8>>,
    image_type: Option<String>,
}

impl MemberResponse {
    fn add_all_children(&mut self, all_members: &[MemberRowWithParents]) {
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
                personal_info: m.personal_info.as_ref().and_then(|p| {
                    p.as_object().map(|o| {
                        o.into_iter()
                            .map(|(k, v)| (k.to_string(), v.as_str().unwrap_or("").to_string()))
                            .rev()
                            .collect::<IndexMap<String, String>>()
                    })
                }),
                children: vec![],
                image: m.image.clone(),
                image_type: m.image_type.clone(),
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
    pub id: i32,
    pub name: String,
    pub gender: Gender,
    pub birthday: Option<DateTime<Utc>>,
    pub last_name: String,
    pub father_id: Option<i32>,
    pub mother_id: Option<i32>,
    pub personal_info: Option<IndexMap<String, String>>,
    pub image: Option<Vec<u8>>,
    pub image_type: Option<String>,
}

use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

#[derive(sqlx::FromRow, Debug, Deserialize, Serialize)]
pub struct Account {
    pub id: Option<u64>,
    pub username: String,
    pub password_hash: String,
}

#[derive(sqlx::FromRow, Debug, Deserialize, Serialize)]
pub struct Post {
    pub id: Option<u64>,
    pub poster_id: u64,
    pub title: String,
    pub body: String,
    pub likes: Option<u64>,
    pub time_stamp: Option<DateTime<Utc>>,
    pub edited: bool
}

#[derive(sqlx::FromRow, Debug, Deserialize, Serialize)]
pub struct Comment {
    pub id: Option<u64>,
    pub post_id: u64,
    pub commenter_id: u64,
    pub body: String,
    pub comment_reply_id: Option<u64>,
    pub likes: Option<u64>,
    pub time_stamp: Option<DateTime<Utc>>,
    pub edited: bool
}

#[derive(sqlx::FromRow, Debug, Deserialize, Serialize)]
pub struct PostLike {
    pub post_id: u64,
    pub account_id: u64,
    pub liked: bool
}

#[derive(sqlx::FromRow, Debug, Deserialize, Serialize)]
pub struct CommentLike {
    pub comment_id: u64,
    pub account_id: u64,
    pub liked: bool
}

#[derive(sqlx::FromRow, Debug, Deserialize, Serialize)]
pub struct AccountID {
    pub account_id: u64
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AccountPasswordUpdate {
    pub account_id: u64,
    pub old: String,
    pub new: String
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PostCommentUpdate {
    pub account_id: u64,
    pub new_body: String
}
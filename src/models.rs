use sqlx::FromRow;
use serde::{Serialize, Deserialize};

#[derive(Debug, Deserialize, FromRow, Serialize)]
pub struct Account {
    pub id: Option<u64>,
    pub username: String,
    pub password_hash: String,
}

#[derive(Debug, Deserialize, FromRow, Serialize)]
pub struct Post {
    pub id: Option<u64>,
    pub poster_id: u64,
    pub title: String,
    pub body: String
}

#[derive(Debug, Deserialize, FromRow, Serialize)]
pub struct Comment {
    pub id: Option<u64>,
    pub post_id: u64,
    pub commenter_id: u64,
    pub body: String,
    pub comment_reply_id: Option<u64>
}

#[derive(Debug, Deserialize, FromRow, Serialize)]
pub struct PostLike {
    pub post_id: u64,
    pub account_id: u64
}

#[derive(Debug, Deserialize, FromRow, Serialize)]
pub struct CommentLike {
    pub comment_id: u64,
    pub account_id: u64
}
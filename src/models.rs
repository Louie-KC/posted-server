use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
/// bool type for MySql Databases. Required for converting TINYINT(1) to bool.
/// 
/// Bool selection in queries must resemble: "<column_name> as `alias: _`"
/// 
/// Reference: https://docs.rs/sqlx/latest/sqlx/macro.query_as.html#column-type-override-infer-from-struct-field
#[derive(sqlx::Type, Debug, Deserialize, Serialize, PartialEq)]
#[sqlx(transparent)]
pub struct MySqlBool (pub bool);

// Request bodies from the user

#[derive(Debug, Deserialize)]
pub struct Account {
    pub username: String,
    pub password: String
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AccountPasswordUpdate {
    pub username: String,
    pub old_password: String,
    pub new_password: String
}

#[derive(Debug, Deserialize)]
pub struct NewPost {
    pub poster_id: u64,
    pub title: String,
    pub body: String
}

#[derive(Debug, Deserialize)]
pub struct NewComment {
    pub post_id: u64,
    pub commenter_id: u64,
    pub comment_reply_id: Option<u64>,
    pub body: String
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PostCommentUpdate {
    pub account_id: u64,
    pub new_body: String
}

// From the DB/To the user

#[derive(sqlx::FromRow, Debug)]
pub struct AccountFromDB {
    pub id: u64,
    pub username: Option<String>,
    pub password_hash: String
}

#[derive(sqlx::FromRow, Debug, Serialize)]
pub struct Post {
    pub id: u64,
    pub poster_id: u64,
    pub title: String,
    pub body: String,
    pub likes: u64,
    pub time_stamp: DateTime<Utc>,
    pub edited: MySqlBool
}

#[derive(sqlx::FromRow, Debug, Serialize)]
pub struct Comment {
    pub id: u64,
    pub post_id: u64,
    pub commenter_id: u64,
    pub body: String,
    pub comment_reply_id: Option<u64>,
    pub likes: u64,
    pub time_stamp: DateTime<Utc>,
    pub edited: MySqlBool
}

// Both to and from user & DB

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

// Aux

#[derive(sqlx::FromRow, Debug, Deserialize, Serialize)]
pub struct AccountID {
    pub account_id: u64
}

use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub struct Post {
    pub id: u64,
    pub poster_id: u64,
    pub title: String,
    pub body: String
}

#[derive(Debug, FromRow)]
pub struct Comment {
    pub id: u64,
    pub post_id: u64,
    pub commenter_id: u64,
    pub body: String,
    pub comment_reply_id: Option<u64>
}

#[derive(FromRow)]
pub struct PostLike {
    pub post_id: u64,
    pub account_id: u64
}

#[derive(FromRow)]
pub struct CommentLike {
    pub comment_id: u64,
    pub account_id: u64
}
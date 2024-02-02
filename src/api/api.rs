use actix_web::{delete, get, post, put, web, HttpResponse};
use actix_web::web::{Data, Json, Path, ServiceConfig};
use actix_web_httpauth::extractors::bearer::BearerAuth;

use serde_json::json;

use crate::database::{database::Database, error::DBError};
use crate::models::*;

pub fn config(config: &mut ServiceConfig) -> () {
    config.service(web::scope("/api")
            .service(create_account)
            .service(login)
            .service(get_posts)
            .service(create_post)
            .service(get_post)
            .service(update_post)
            .service(delete_post)
            .service(get_post_comments)
            .service(make_post_comment)
            .service(update_comment)
            .service(delete_comment)
            .service(get_user_posts)
            .service(get_user_comments)
            .service(vote_on_post)
            .service(vote_on_comment)
        );
}

#[post("/register")]
pub async fn create_account(db: Data<Database>, account: Json<Account>) -> HttpResponse {
    if account.username.is_empty() {
        return HttpResponse::BadRequest().reason("The provided username was empty").finish();
    }
    if account.password_hash.is_empty() {
        return HttpResponse::BadRequest().reason("The provided password hash was empty").finish();
    }

    let result = db.create_account(&account.username, &account.password_hash).await;
    match result {
        Ok(()) => HttpResponse::Ok().json(json!({"status": "Success"})),
        Err(DBError::UnexpectedRowsAffected(_, _)) => {
            HttpResponse::BadRequest().reason("Username is taken").finish()
        }
        Err(_) => HttpResponse::InternalServerError().finish()
    }
}

#[post("authenticate")]
pub async fn login(db: Data<Database>, data: Json<Account>) -> HttpResponse {
    if data.username.is_empty() {
        return HttpResponse::BadRequest().reason("The provided username was empty").finish()
    }
    if data.password_hash.is_empty() {
        return HttpResponse::BadRequest().reason("The provided password hash was empty").finish()
    }

    // TODO: Actual token generation and storage

    let account = Account { id: None, username: data.username.clone(), password_hash: data.password_hash.clone() };
    let id_result = db.read_account_id(account).await;

    match id_result {
        Ok(id) => {
            HttpResponse::Ok().json(json!({"id": id, "token": id}))  // TODO: Add actual token
        },
        Err(_) => HttpResponse::BadRequest().finish()
    }
}

#[get("/posts")]
pub async fn get_posts(db: Data<Database>) -> HttpResponse {
    let result = db.read_posts(64).await;
    match result {
        Ok(posts) => HttpResponse::Ok().json(posts),
        Err(_) => HttpResponse::InternalServerError().finish()
    }
}

#[post("/posts")]
pub async fn create_post(db: Data<Database>, data: Json<Post>, auth: BearerAuth) -> HttpResponse {
    if data.title.is_empty() {
        return HttpResponse::BadRequest().reason("Post has no title").finish()
    }
    if data.body.is_empty() {
        return HttpResponse::BadRequest().reason("Post has no body/content").finish()
    }

    // TODO: Proper auth token check
    if auth.token().ne(&data.poster_id.to_string()) {
        return HttpResponse::Unauthorized().reason("Invalid authorization token").finish()
    }

    let post = Post { id: None, poster_id: data.poster_id, title: data.title.clone(), body: data.body.clone(), likes: None };
    let result = db.create_post(post).await;
    match result {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish()
    }
}

#[get("/posts/{post_id}")]
pub async fn get_post(db: Data<Database>, path: Path<String>) -> HttpResponse {
    let post_id = match path.parse::<u64>() {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().reason("Invalid post_id format").finish()
    };

    let result = db.read_post_by_id(post_id).await;
    match result {
        Ok(post) => HttpResponse::Ok().json(post),
        Err(DBError::NoResult) => HttpResponse::BadRequest().reason("Invalid post_id").finish(),
        Err(_) => HttpResponse::InternalServerError().finish()
    }
}

#[put("/posts/{post_id}")]
pub async fn update_post(
    db: Data<Database>,
    path: Path<String>,
    data: Json<PostCommentUpdate>,
    auth: BearerAuth
) -> HttpResponse {
    let post_id = match path.parse::<u64>() {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().reason("Invalid post_id format").finish()
    };

    let post = match db.read_post_by_id(post_id).await {
        Ok(p)  => p,
        Err(_) => return HttpResponse::BadRequest().reason("Invalid post_id").finish()
    };

    // TODO: Proper auth token check
    if auth.token().ne(&post.poster_id.to_string()) {
        return HttpResponse::Unauthorized().reason("Invalid authorization token").finish()
    }

    match db.update_post_body(post_id, data.new_body.clone()).await {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish()
    }
}

#[delete("/posts/{post_id}")]
pub async fn delete_post(db: Data<Database>, path: Path<String>, auth: BearerAuth) -> HttpResponse {
    let post_id = match path.parse::<u64>() {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().reason("Invalid post_id format").finish()
    };

    let post = match db.read_post_by_id(post_id).await {
        Ok(p) => p,
        Err(_) => return HttpResponse::BadRequest().reason("Invalid post_id").finish()
    };

    // TODO: Proper auth token check
    if auth.token().ne(&post.poster_id.to_string()) {
        return HttpResponse::Unauthorized().reason("Invalid authorization token").finish()
    }

    let result = db.delete_post(post_id).await;
    match result {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish()
    }
}

#[get("/posts/{post_id}/comments")]
pub async fn get_post_comments(db: Data<Database>, path: Path<String>) -> HttpResponse {
    let post_id = match path.parse::<u64>() {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().reason("Invalid post_id format").finish()
    };
    let result = db.read_comments_of_post(post_id).await;
    match result {
        Ok(comments) => HttpResponse::Ok().json(comments),
        Err(_) => HttpResponse::InternalServerError().finish()
    }
}

#[post("/comment")]
pub async fn make_post_comment(db: Data<Database>, data: Json<Comment>, auth: BearerAuth) -> HttpResponse {
    if data.body.is_empty() {
        return HttpResponse::BadRequest().reason("Comment without body").finish()
    }
    // TODO: Proper auth token check
    if auth.token().ne(&data.commenter_id.to_string()) {
        return HttpResponse::Unauthorized().reason("Invalid authorization token").finish()
    }

    let comment = Comment { id: None, post_id: data.post_id,
        commenter_id: data.commenter_id, body: data.body.clone(),
        comment_reply_id: data.comment_reply_id, likes: None };
    let result = db.create_comment(comment).await;
    match result {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(DBError::UnexpectedRowsAffected(_, _)) => {
            HttpResponse::BadRequest().reason("Comment data was invalid").finish()
        },
        Err(_) => HttpResponse::InternalServerError().finish()
    }
}

#[put("/comment/{comment_id}")]
pub async fn update_comment(
    db: Data<Database>,
    path: Path<String>,
    data: Json<PostCommentUpdate>,
    auth: BearerAuth
) -> HttpResponse {
    let comment_id = match path.parse::<u64>() {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().reason("Invalid comment_id format").finish()
    };

    let comment = match db.read_comment_by_id(comment_id).await {
        Ok(c)  => c,
        Err(_) => return HttpResponse::BadRequest().reason("Invalid comment_id").finish()
    };

    // TODO: Proper auth token check
    if auth.token().ne(&comment.poster_id.to_string()) {
        return HttpResponse::Unauthorized().reason("Invalid authorization token").finish()
    }

    match db.update_comment_body(comment_id, data.new_body.clone()).await {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish()
    }
}

#[delete("/comment/{comment_id}")]
pub async fn delete_comment(db: Data<Database>, path: Path<String>, auth: BearerAuth) -> HttpResponse {
    let comment_id = match path.parse::<u64>() {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().reason("Invalid comment_id format").finish()
    };

    let comment = match db.read_comment_by_id(comment_id).await {
        Ok(c)  => c,
        Err(_) => return HttpResponse::BadRequest().reason("Invalid comment_id").finish()
    };

    // TODO: Proper auth token check
    if auth.token().ne(&comment.poster_id.to_string()) {
        return HttpResponse::Unauthorized().reason("Invalid authorization token").finish()
    }

    let result = db.delete_comment(comment_id).await;
    match result {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish()
    }
}

#[get("/users/{user_id}/posts")]
pub async fn get_user_posts(db: Data<Database>, path: Path<String>) -> HttpResponse {
    let user_id = match path.parse::<u64>() {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().reason("Invalid user_id format").finish()
    };
    let result = db.read_posts_by_user(user_id).await;
    match result {
        Ok(posts) => HttpResponse::Ok().json(posts),
        Err(_) => HttpResponse::InternalServerError().finish()
    }
}

#[get("/users/{user_id}/comments")]
pub async fn get_user_comments(db: Data<Database>, path: Path<String>) -> HttpResponse {
    let user_id = match path.parse::<u64>() {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().reason("Invalid user_id format").finish()
    };
    let result = db.read_comments_by_user(user_id).await;
    match result {
        Ok(comments) => HttpResponse::Ok().json(comments),
        Err(_) => HttpResponse::InternalServerError().finish()
    }
}

#[post("/vote/post")]
pub async fn vote_on_post(db: Data<Database>, data: Json<PostLike>, auth: BearerAuth) -> HttpResponse {
    if data.account_id == 0 || data.post_id == 0 {
        return HttpResponse::BadRequest().finish()
    }

    // TODO: Replace with proper auth token check
    if auth.token().ne(&data.account_id.to_string()) {
        return HttpResponse::Unauthorized().finish()
    }

    let result = match data.liked {
        true  => db.create_post_like(data.post_id, data.account_id).await,
        false => db.delete_post_like(data.post_id, data.account_id).await
    };
    match result {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(DBError::UnexpectedRowsAffected(_, _)) => HttpResponse::AlreadyReported().finish(),
        Err(_) => HttpResponse::InternalServerError().finish()
    }
}

#[post("/vote/comment")]
pub async fn vote_on_comment(db: Data<Database>, data: Json<CommentLike>, auth: BearerAuth) -> HttpResponse {
    if data.account_id == 0 || data.comment_id == 0 {
        return HttpResponse::BadRequest().finish()
    }

    // TODO: Replace with proper auth token check
    if auth.token().ne(&data.account_id.to_string()) {
        return HttpResponse::Unauthorized().finish()
    }

    let result = match data.liked {
        true  => db.create_comment_like(data.comment_id, data.account_id).await,
        false => db.delete_comment_like(data.comment_id, data.account_id).await
    };
    match result {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(DBError::UnexpectedRowsAffected(_, _)) => HttpResponse::AlreadyReported().finish(),
        Err(_) => HttpResponse::InternalServerError().finish()
    }
}
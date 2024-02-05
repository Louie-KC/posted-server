use std::sync::Mutex;

use actix_web::{delete, get, post, put, web, HttpResponse};
use actix_web::web::{Data, Json, Path, ServiceConfig};
use actix_web_httpauth::extractors::bearer::BearerAuth;

use serde_json::json;

use crate::database::{database::Database, error::DBError};
use crate::models::*;
use crate::auth::auth::AuthService;

pub fn config(config: &mut ServiceConfig) -> () {
    config.service(web::scope("/api")
            .service(create_account)
            .service(login)
            .service(change_password)
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

#[post("/account/register")]
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

#[post("/account/authenticate")]
pub async fn login(
    db: Data<Database>,
    auth: Data<Mutex<AuthService>>,
    data: Json<Account>
) -> HttpResponse {
    if data.username.is_empty() {
        return HttpResponse::BadRequest().reason("The provided username was empty").finish()
    }
    if data.password_hash.is_empty() {
        return HttpResponse::BadRequest().reason("The provided password hash was empty").finish()
    }

    let account = Account { id: None, username: data.username.clone(), password_hash: data.password_hash.clone() };
    let id_result = db.read_account_id(account).await;

    let mut auth_service = match auth.try_lock() {
        Ok(service) => service,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    match id_result {
        Ok(id) => {
            let token = auth_service.generate_for_user(id);
            HttpResponse::Ok().json(json!({"id": id, "token": token}))
        },
        Err(_) => HttpResponse::BadRequest().finish()
    }
}

#[put("/account/change_password")]
pub async fn change_password(
    db: Data<Database>,
    data: Json<AccountPasswordUpdate>,
    auth: Data<Mutex<AuthService>>,
    bearer: BearerAuth
) -> HttpResponse {
    if data.new.eq(&data.old) {
        return HttpResponse::BadRequest().reason("Old and new are identical").finish();
    }

    if let Err(bad_token_response) = verify_token(data.account_id, bearer.token(), auth) {
        return bad_token_response;
    }

    match db.update_account_password(data.account_id, &data.old, &data.new).await {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(DBError::UnexpectedRowsAffected(1, 0)) => HttpResponse::BadRequest().finish(),
        Err(_) => HttpResponse::InternalServerError().finish()
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
pub async fn create_post(
    db: Data<Database>,
    data: Json<Post>,
    auth: Data<Mutex<AuthService>>,
    bearer: BearerAuth
) -> HttpResponse {
    if data.title.is_empty() {
        return HttpResponse::BadRequest().reason("Post has no title").finish()
    }
    if data.body.is_empty() {
        return HttpResponse::BadRequest().reason("Post has no body/content").finish()
    }

    if let Err(bad_token_response) = verify_token(data.poster_id, bearer.token(), auth) {
        return bad_token_response;
    }

    let post = Post { 
        id: None, poster_id: data.poster_id, title: data.title.clone(),
        body: data.body.clone(), likes: None, time_stamp: None, edited: Some(false)
    };
    
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
    auth: Data<Mutex<AuthService>>,
    bearer: BearerAuth
) -> HttpResponse {
    let post_id = match path.parse::<u64>() {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().reason("Invalid post_id format").finish()
    };

    // let post = match db.read_post_by_id(post_id).await {
    //     Ok(p)  => p,
    //     Err(_) => return HttpResponse::BadRequest().reason("Invalid post_id").finish()
    // };

    // TODO: Capture the above invalid post_id error again            !!!

    if let Err(bad_token_response) = verify_token(data.account_id, bearer.token(), auth) {
        return bad_token_response;
    }

    match db.update_post_body(post_id, data.new_body.clone()).await {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish()
    }
}

#[delete("/posts/{post_id}")]
pub async fn delete_post(
    db: Data<Database>,
    path: Path<String>,
    data: Json<AccountID>,
    auth: Data<Mutex<AuthService>>,
    bearer: BearerAuth
) -> HttpResponse {
    let post_id = match path.parse::<u64>() {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().reason("Invalid post_id format").finish()
    };

    // let post = match db.read_post_by_id(post_id).await {
    //     Ok(p) => p,
    //     Err(_) => return HttpResponse::BadRequest().reason("Invalid post_id").finish()
    // };

    // TODO: Capture the above invalid post_id error again            !!!

    if let Err(bad_token_response) = verify_token(data.account_id, bearer.token(), auth) {
        return bad_token_response;
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
pub async fn make_post_comment(
    db: Data<Database>,
    data: Json<Comment>,
    auth: Data<Mutex<AuthService>>,
    bearer: BearerAuth
) -> HttpResponse {
    if data.body.is_empty() {
        return HttpResponse::BadRequest().reason("Comment without body").finish()
    }

    if let Err(bad_token_response) = verify_token(data.commenter_id, bearer.token(), auth) {
        return bad_token_response;
    }

    let comment = Comment { id: None, post_id: data.post_id,
        commenter_id: data.commenter_id, body: data.body.clone(),
        comment_reply_id: data.comment_reply_id, likes: None, time_stamp: None, edited: Some(false)
    };
    
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
    auth: Data<Mutex<AuthService>>,
    bearer: BearerAuth
) -> HttpResponse {
    let comment_id = match path.parse::<u64>() {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().reason("Invalid comment_id format").finish()
    };

    // let comment = match db.read_comment_by_id(comment_id).await {
    //     Ok(c)  => c,
    //     Err(_) => return HttpResponse::BadRequest().reason("Invalid comment_id").finish()
    // };

    // TODO: Capture the above invalid comment_id response again        !!!!!

    if let Err(bad_token_response) = verify_token(data.account_id, bearer.token(), auth) {
        return bad_token_response;
    }

    match db.update_comment_body(comment_id, data.new_body.clone()).await {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish()
    }
}

#[delete("/comment/{comment_id}")]
pub async fn delete_comment(
    db: Data<Database>,
    path: Path<String>,
    data: Json<AccountID>,
    auth: Data<Mutex<AuthService>>,
    bearer: BearerAuth
) -> HttpResponse {
    let comment_id: u64 = match path.parse::<u64>() {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().reason("Invalid comment_id format").finish()
    };

    // let comment = match db.read_comment_by_id(comment_id).await {
    //     Ok(c)  => c,
    //     Err(_) => return HttpResponse::BadRequest().reason("Invalid comment_id").finish()
    // };

    // TODO: Capture the above invalid comment_id response again        !!!!!

    if let Err(bad_token_response) = verify_token(data.account_id, bearer.token(), auth) {
        return bad_token_response;
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
pub async fn vote_on_post(
    db: Data<Database>,
    data: Json<PostLike>,
    auth: Data<Mutex<AuthService>>,
    bearer: BearerAuth
) -> HttpResponse {
    if data.account_id == 0 || data.post_id == 0 {
        return HttpResponse::BadRequest().finish()
    }

    if let Err(bad_token_response) = verify_token(data.account_id, bearer.token(), auth) {
        return bad_token_response;
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
pub async fn vote_on_comment(
    db: Data<Database>,
    data: Json<CommentLike>,
    auth: Data<Mutex<AuthService>>,
    bearer: BearerAuth
) -> HttpResponse {
    if data.account_id == 0 || data.comment_id == 0 {
        return HttpResponse::BadRequest().finish()
    }

    if let Err(bad_token_response) = verify_token(data.account_id, bearer.token(), auth) {
        return bad_token_response;
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

/// Check that a `token_str` is valid for an `account_id` in the `auth` AuthService.
/// 
/// Note: The MutexGuard for AuthService that is acquired is dropped at the end
///       of the function, releasing the lock on the AuthService.
pub fn verify_token(
    account_id: u64,
    token_str: &str,
    auth: Data<Mutex<AuthService>>
) -> Result<(), HttpResponse> {
    let auth_service_guard = match auth.try_lock() {
        Ok(service) => service,
        Err(_) => return Err(HttpResponse::InternalServerError().finish())
    };
    match auth_service_guard.validate_str(account_id, token_str) {
        Ok(true)  => Ok(()),
        Ok(false) => Err(HttpResponse::Unauthorized().finish()),
        Err(_)    => Err(HttpResponse::Unauthorized().reason("Invalid token").finish())
    }
}
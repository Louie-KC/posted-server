use std::sync::Mutex;

use actix_web::{delete, get, post, put, web, HttpResponse};
use actix_web::web::{Data, Json, Path, ServiceConfig};
use actix_web_httpauth::extractors::bearer::BearerAuth;

use log::warn;
use serde_json::json;

use crate::auth::auth::AuthService;
// use crate::cache::cache::Cache;
use crate::database::{database::Database, error::DBError};
use crate::models::*;
// use crate::auth::auth::AuthService;
// use crate::auth::redis_auth;

use argon2::{
    password_hash::{
        rand_core::OsRng,
        PasswordHash, PasswordHasher, PasswordVerifier, SaltString
    },
    Argon2
};

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
pub async fn create_account(
    db: Data<Database>,
    argon2: Data<Argon2<'_>>,
    account: Json<Account>
) -> HttpResponse {
    if account.username.is_empty() {
        return HttpResponse::BadRequest().reason("The provided username was empty").finish();
    }
    if account.password.is_empty() {
        return HttpResponse::BadRequest().reason("The provided password hash was empty").finish();
    }

    let username = account.username.clone();
    let salt = SaltString::generate(&mut OsRng);
    let pw_hash = match argon2.hash_password(account.password.as_bytes(), &salt) {
        Ok(hash) => hash.to_string(),
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };
    std::mem::drop(account);  // TODO: Zeroize Account struct or just the password
    std::mem::drop(salt);

    let result = db.create_account(&username, &pw_hash).await;
    match result {
        Ok(()) => HttpResponse::Ok().json(json!({"status": "Success"})),
        Err(DBError::UnexpectedRowsAffected { expected: 1, actual: 0 } ) => {
            HttpResponse::BadRequest().reason("Username is taken").finish()
        }
        Err(_) => HttpResponse::InternalServerError().finish()
    }
}

#[post("/account/authenticate")]
pub async fn login(
    db: Data<Database>,
    auth: Data<Mutex<AuthService>>,
    argon2: Data<Argon2<'_>>,
    data: Json<Account>
) -> HttpResponse {
    if data.username.is_empty() {
        return HttpResponse::BadRequest().reason("The provided username was empty").finish()
    }
    if data.password.is_empty() {
        return HttpResponse::BadRequest().reason("The provided password was empty").finish()
    }

    let account_details = match db.read_account_by_username(&data.username).await{
        Ok(details) => details,
        Err(DBError::NoResult) => return HttpResponse::BadRequest().reason("Username doesn't exist").finish(),
        Err(_) => return HttpResponse::InternalServerError().finish()
    };

    let parsed_pw_hash = match PasswordHash::new(&account_details.password_hash) {
        Ok(parsed) => parsed,
        Err(_) => {
            warn!("login: PasswordHash could not be created for user '{}'", data.username);
            return HttpResponse::InternalServerError().finish()
        }
    };

    match argon2.verify_password(data.password.as_bytes(), &parsed_pw_hash) {
        Ok(()) => {
            let token = auth.lock().unwrap().generate_user_token(account_details.id).await;
            HttpResponse::Ok().json(json!({"id": account_details.id, "token": token}))
        },
        Err(_) => HttpResponse::BadRequest().finish()
    }
}

#[put("/account/change_password")]
pub async fn change_password(
    db: Data<Database>,
    auth: Data<Mutex<AuthService>>,
    argon2: Data<Argon2<'_>>,
    bearer: BearerAuth,
    data: Json<AccountPasswordUpdate>
) -> HttpResponse {
    if data.old_password.is_empty() || data.new_password.is_empty() {
        return HttpResponse::BadRequest().reason("One or both passwords are empty").finish()
    }
    if data.new_password.eq(&data.old_password) {
        return HttpResponse::BadRequest().reason("Old and new are identical").finish();
    }

    // Copy/use necessary data and then drop
    let username: String = data.username.clone();
    let old_pw = data.old_password.clone();
    let salt = SaltString::generate(&mut OsRng);
    let new_pw_hash = match argon2.hash_password(data.new_password.as_bytes(), &salt) {
        Ok(hash) => hash,
        Err(_) => return HttpResponse::InternalServerError().finish()
    };
    std::mem::drop(data);  // TODO: Zeroize struct or just new and old passwords

    let old_account_details = match db.read_account_by_username(&username).await {
        Ok(account_details) => account_details,
        Err(DBError::NoResult) => return HttpResponse::BadRequest().reason("Username does not exist").finish(),
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    if let Err(err_response) = verify_token(old_account_details.id, bearer.token(), auth).await {
        return err_response;
    }

    let old_pw_hash = match PasswordHash::new(&old_account_details.password_hash) {
        Ok(hash) => hash,
        Err(_) => return HttpResponse::InternalServerError().finish()
    };
    
    if argon2.verify_password(old_pw.as_bytes(), &old_pw_hash).is_err() {
        return HttpResponse::BadRequest().reason("Invalid old password").finish()
    }
    std::mem::drop(old_pw);  // TODO: Zeroize struct or just new and old passwords

    match db.update_account_password(old_account_details.id, &old_account_details.password_hash, &new_pw_hash.to_string()).await {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(DBError::UnexpectedRowsAffected{ expected: 1, actual: 0 }) => {
            HttpResponse::BadRequest().finish()
        },
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

    if let Err(err_response) = verify_token(data.poster_id, bearer.token(), auth).await {
        return err_response;
    }

    let post = Post { 
        id: None, poster_id: data.poster_id, title: data.title.clone(),
        body: data.body.clone(), likes: None, time_stamp: None, edited: Some(MySqlBool(false))
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

    if let Err(err_response) = verify_token(data.account_id, bearer.token(), auth).await {
        return err_response;
    }

    match db.update_post_body(post_id, data.new_body.clone()).await {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(DBError::UnexpectedRowsAffected{ expected: 1, actual: 0 }) => {
            HttpResponse::BadRequest().reason("Invalid post_id").finish()
        },
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

    if let Err(err_response) = verify_token(data.account_id, bearer.token(), auth).await {
        return err_response;
    }

    let result = db.delete_post(post_id).await;
    match result {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(DBError::UnexpectedRowsAffected{ expected: 1, actual: 0 }) => {
            HttpResponse::BadRequest().reason("Invalid post_id").finish()
        },
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

    if let Err(err_response) = verify_token(data.commenter_id, bearer.token(), auth).await {
        return err_response;
    }

    let comment = Comment { id: None, post_id: data.post_id,
        commenter_id: data.commenter_id, body: data.body.clone(),
        comment_reply_id: data.comment_reply_id, likes: None, time_stamp: None, edited: Some(MySqlBool(false))
    };
    
    let result = db.create_comment(comment).await;
    match result {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(DBError::UnexpectedRowsAffected{ expected: 1, actual: 0 }) => {
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

    if let Err(err_response) = verify_token(data.account_id, bearer.token(), auth).await {
        return err_response;
    }

    match db.update_comment_body(comment_id, data.new_body.clone()).await {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(DBError::UnexpectedRowsAffected{ expected: 1, actual: 0 }) => {
            HttpResponse::BadRequest().reason("Invalid comment_id").finish()
        },
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

    if let Err(err_response) = verify_token(data.account_id, bearer.token(), auth).await {
        return err_response;
    }

    // Mark post as "deleted" by overwriting the body
    let result = db.update_comment_body(comment_id, "[DELETED]".to_string()).await;
    match result {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(DBError::UnexpectedRowsAffected{ expected: 1, actual: 0 }) => {
            HttpResponse::BadRequest().reason("Invalid comment_id").finish()
        },
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

    if let Err(err_response) = verify_token(data.account_id, bearer.token(), auth).await {
        return err_response;
    }

    let result = match data.liked {
        true  => db.create_post_like(data.post_id, data.account_id).await,
        false => db.delete_post_like(data.post_id, data.account_id).await
    };
    match result {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(DBError::UnexpectedRowsAffected{ expected: 1, actual: 0 }) => {
            HttpResponse::AlreadyReported().finish()
        },
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

    if let Err(err_response) = verify_token(data.account_id, bearer.token(), auth).await {
        return err_response;
    }

    let result = match data.liked {
        true  => db.create_comment_like(data.comment_id, data.account_id).await,
        false => db.delete_comment_like(data.comment_id, data.account_id).await
    };
    match result {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(DBError::UnexpectedRowsAffected{ expected: 1, actual: 0 }) => {
            HttpResponse::AlreadyReported().finish()
        },
        Err(_) => HttpResponse::InternalServerError().finish()
    }
}

/// Check that a `token_str` is valid for an `account_id` in the `auth` AuthService.
/// 
/// Note: The MutexGuard for AuthService that is acquired is dropped at the end
///       of the function, releasing the lock on the AuthService.
pub async fn verify_token(
    account_id: u64,
    token_str: &str,
    auth: Data<Mutex<AuthService>>
) -> Result<(), HttpResponse> {
    match auth.lock().unwrap().validate(account_id, token_str).await {
        Ok(true)  => Ok(()),
        Ok(false) => Err(HttpResponse::Unauthorized().finish()),
        Err(_)    => Err(HttpResponse::Unauthorized().reason("Invalid token").finish()),
    }
}
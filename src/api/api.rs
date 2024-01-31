use actix_web::{get, post, web, HttpResponse};
use actix_web::web::{Data, Json, Path, ServiceConfig};
use actix_web_httpauth::extractors::bearer::BearerAuth;

use serde_json::json;

use crate::database::database::{Database, DBError};
use crate::models::*;

pub fn config(config: &mut ServiceConfig) -> () {
    config.service(web::scope("/api")
            .service(create_account)
            .service(login)
            .service(get_posts)
            .service(create_post)
            .service(get_post_comments)
            .service(make_post_comment)
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
        Err(DBError::SQLXError(_)) => {
            HttpResponse::InternalServerError().finish()
        }
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
    if auth.token().ne(&format!("{}", data.poster_id)) {
        return HttpResponse::Unauthorized().reason("Invalid authorization token").finish()
    }

    let post = Post { id: None, poster_id: data.poster_id, title: data.title.clone(), body: data.body.clone() };
    let result = db.create_post(post).await;
    match result {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish()
    }
}

#[get("/{post_id}/comments")]
pub async fn get_post_comments(db: Data<Database>, path: Path<String>) -> HttpResponse {
    let post_id = match path.parse::<u64>() {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().reason("Invalid post_id").finish()
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
    if auth.token().ne(&format!("{}", data.commenter_id)) {
        return HttpResponse::Unauthorized().reason("Invalid authorization token").finish()
    }

    let comment = Comment { id: None, post_id: data.post_id,
        commenter_id: data.commenter_id, body: data.body.clone(),
        comment_reply_id: data.comment_reply_id };
    let result = db.create_comment(comment).await;
    match result {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(DBError::UnexpectedRowsAffected(_, _)) => {
            HttpResponse::BadRequest().reason("Comment data was invalid").finish()
        },
        Err(DBError::SQLXError(_)) => HttpResponse::InternalServerError().finish()
    }
}
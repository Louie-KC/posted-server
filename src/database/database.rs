use log::warn;
use sqlx::{MySql, Pool, Row};
use sqlx::mysql::{MySqlPoolOptions, MySqlQueryResult};

use crate::models::{AccountFromDB, Comment, NewComment, NewPost, Post};
use crate::database::error::DBError;

type DBResult<T> = Result<T, DBError>;

pub struct Database {
    conn_pool: Pool<MySql>
}

impl Database {
    pub async fn new(url: &str) -> Self {
        let pool = MySqlPoolOptions::new().connect(url)
            .await
            .expect("Failed to connect to the database");
        Database { conn_pool: pool }
    }

    // Create

    pub async fn create_account(&self, username: &str, password_hash: &str) -> DBResult<()> {
        match sqlx::query("INSERT INTO Account (username, password_hash) VALUES (?, ?);")
            .bind(username)
            .bind(password_hash)
            .execute(&self.conn_pool)
            .await
        {
            Ok(res) => expected_rows_affected(res, 1),
            Err(e) => Err(log_error(DBError::from(e)))
        }
    }

    pub async fn create_post(&self, post: NewPost) -> DBResult<()> {
        match sqlx::query("INSERT INTO Post (poster_id, title, body) VALUES (?, ?, ?);")
            .bind(post.poster_id)
            .bind(post.title)
            .bind(post.body)
            .execute(&self.conn_pool)
            .await
        {
            Ok(res) => expected_rows_affected(res, 1),
            Err(e) => Err(log_error(DBError::from(e)))
        }
    }

    pub async fn create_comment(&self, comment: NewComment) -> DBResult<()> {
        match sqlx::query("INSERT INTO Comment (post_id, commenter_id, body, comment_reply_id) VALUES (?, ?, ?, ?);")
            .bind(comment.post_id)
            .bind(comment.commenter_id)
            .bind(comment.body)
            .bind(comment.comment_reply_id)
            .execute(&self.conn_pool)
            .await
        {
            Ok(res) => expected_rows_affected(res, 1),
            Err(e) => Err(log_error(DBError::from(e)))
        }
    }

    pub async fn create_post_like(&self, post_id: u64, account_id: u64) -> DBResult<()> {
        match sqlx::query("INSERT IGNORE INTO PostLike (post_id, account_id) values (?, ?);")
            .bind(post_id)
            .bind(account_id)
            .execute(&self.conn_pool)
            .await
        {
            Ok(res) => expected_rows_affected(res, 1),
            Err(e) => Err(log_error(DBError::from(e)))
        }
    }

    pub async fn create_comment_like(&self, comment_id: u64, account_id: u64) -> DBResult<()> {
        match sqlx::query("INSERT IGNORE INTO CommentLike (comment_id, account_id) values (?, ?);")
            .bind(comment_id)
            .bind(account_id)
            .execute(&self.conn_pool)
            .await
        {
            Ok(res) => expected_rows_affected(res, 1),
            Err(e) => Err(log_error(DBError::from(e)))
        }
    }

    // Read

    pub async fn _read_account_by_id(&self, id: u64) -> DBResult<AccountFromDB> {
        // TODO, avoid cast and return null for an None for id
        let result = sqlx::query_as!(AccountFromDB,
            "SELECT CAST(0 AS UNSIGNED) as 'id', username, password_hash
            FROM Account
            WHERE id = ?
            LIMIT 1;", id)
            .fetch_one(&self.conn_pool)
            .await;

        match result {
            Ok(acc) => Ok(acc),
            Err(e) => Err(log_error(DBError::from(e)))
        }
    }

    pub async fn read_account_by_username(&self, username: &str) -> DBResult<AccountFromDB> {
        let result = sqlx::query_as!(AccountFromDB,
            "SELECT CAST(id AS UNSIGNED) as 'id', '' as 'username', password_hash
            FROM Account
            WHERE username = ?
            LIMIT 1;", username)
            .fetch_one(&self.conn_pool)
            .await;
        
        match result {
            Ok(acc) => Ok(acc),
            Err(e) => Err(log_error(DBError::from(e)))
        }
    }

    pub async fn read_posts(&self, max_posts: u64) -> DBResult<Vec<Post>> {
        let result = sqlx::query_as!(Post,
            "SELECT p.id, p.poster_id, p.title, p.body, p.time_stamp, p.edited as `edited: _`,
                CAST(count(pl.account_id) AS UNSIGNED) AS 'likes'
            FROM Post p
            LEFT JOIN PostLike pl
            ON p.id = pl.post_id
            GROUP BY p.id
            LIMIT ?;", max_posts)
            .fetch_all(&self.conn_pool)
            .await;
        match result {
            Ok(posts) => Ok(posts),
            Err(e)  => Err(log_error(DBError::from(e)))
        }
    }

    pub async fn read_post_by_id(&self, post_id: u64) -> DBResult<Post> {
        let result = sqlx::query_as!(Post,
            "SELECT p.id, p.poster_id, p.title, p.body, p.time_stamp, p.edited as `edited: _`,
                CAST(count(pl.account_id) AS UNSIGNED) AS 'likes'
            FROM Post p
            LEFT JOIN PostLike pl
            ON p.id = pl.post_id
            WHERE p.id = ?
            GROUP BY p.id;", post_id)
            .fetch_one(&self.conn_pool)
            .await;
        match result {
            Ok(post) => Ok(post),
            Err(e) => Err(DBError::from(e))
        }
    }

    pub async fn read_posts_by_user(&self, user_id: u64) -> DBResult<Vec<Post>> {
        let result = sqlx::query_as!(Post,
            "SELECT p.id, p.poster_id, p.title, p.body, p.time_stamp,
                p.edited as `edited: _`,
                CAST(count(pl.account_id) AS UNSIGNED) AS 'likes'
            FROM Post p
            LEFT JOIN PostLike pl
            ON p.id = pl.post_id
            WHERE p.poster_id = ?
            GROUP BY p.id;", user_id)
            .fetch_all(&self.conn_pool)
            .await;
        match result {
            Ok(posts) => Ok(posts),
            Err(e) => Err(log_error(DBError::from(e)))
        }
    }

    pub async fn read_comments_of_post(&self, post_id: u64) -> DBResult<Vec<Comment>> {
        let result = sqlx::query_as!(Comment,
            "SELECT c.id, c.post_id, c.commenter_id, c.body, c.comment_reply_id,
                c.time_stamp, c.edited as `edited: _`,
                CAST(count(cl.comment_id) AS UNSIGNED) AS 'likes'
            FROM Comment c
            LEFT JOIN CommentLike cl
            ON c.id = cl.comment_id
            WHERE c.post_id = ?
            GROUP BY c.id", post_id)
            .fetch_all(&self.conn_pool)
            .await;


        match result {
            Ok(comments) => Ok(comments),
            Err(e) => Err(log_error(DBError::from(e)))
        }
    }

    pub async fn read_comments_by_user(&self, user_id: u64) -> DBResult<Vec<Comment>> {
        let result = sqlx::query_as!(Comment,
            "SELECT c.id, c.post_id, c.commenter_id, c.body, c.comment_reply_id,
                c.time_stamp, c.edited as `edited: _`,
                CAST(count(cl.comment_id) AS UNSIGNED) AS 'likes'
            FROM Comment c
            LEFT JOIN CommentLike cl
            ON c.id = cl.comment_id
            WHERE c.commenter_id = ?
            GROUP BY c.id", user_id)
            .fetch_all(&self.conn_pool)
            .await;

        match result {
            Ok(comments) => Ok(comments),
            Err(e) => Err(log_error(DBError::from(e)))
        }
    }

    pub async fn _read_post_likes(&self, post_id: u64) -> DBResult<u64> {
        let result = sqlx::query(
            "SELECT CAST(count(post_id) AS UNSIGNED)
            FROM PostLike
            WHERE post_id = ?;")
            .bind(post_id)
            .fetch_one(&self.conn_pool)
            .await;
        match result {
            Ok(row) => Ok(row.try_get(0)?),
            Err(e) => Err(log_error(DBError::from(e)))
        }
    }

    pub async fn _read_comment_likes(&self, comment_id: u64) -> DBResult<u64> {
        let result = sqlx::query(
            "SELECT CAST(count(post_id) AS UNSIGNED)
            FROM CommentLike
            WHERE comment_id = ?;")
            .bind(comment_id)
            .fetch_one(&self.conn_pool)
            .await;
        match result {
            Ok(row) => Ok(row.try_get(0)?),
            Err(e) => Err(log_error(DBError::from(e)))
        }
    }

    // Update

    pub async fn update_account_password(&self, account_id: u64, old: &str, new: &str) -> DBResult<()> {
        let result = sqlx::query(
            "UPDATE Account
            SET password_hash = ?
            WHERE id = ?
            AND password_hash = ?;")
            .bind(new)
            .bind(account_id)
            .bind(old)
            .execute(&self.conn_pool)
            .await;
    
        match result {
            Ok(res) => expected_rows_affected(res, 1),
            Err(err) => Err(log_error(DBError::from(err)))
        }
    }

    pub async fn update_post_body(&self, post_id: u64, new_body: String) -> DBResult<()> {
        let result = sqlx::query(
            "UPDATE Post
            SET body = ?, edited = true
            WHERE id = ?")
            .bind(new_body)
            .bind(post_id)
            .execute(&self.conn_pool)
            .await;
        
        match result {
            Ok(res) => expected_rows_affected(res, 1),
            Err(err) => Err(log_error(DBError::from(err)))
        }
    }

    pub async fn update_comment_body(&self, comment_id: u64, new_body: String) -> DBResult<()> {
        let result = sqlx::query(
            "UPDATE Comment
            SET body = ?, edited = true
            WHERE id = ?")
            .bind(new_body)
            .bind(comment_id)
            .execute(&self.conn_pool)
            .await;
        
        match result {
            Ok(res) => expected_rows_affected(res, 1),
            Err(err) => Err(log_error(DBError::from(err)))
        }
    }

    // Delete

    pub async fn delete_post(&self, post_id: u64) -> DBResult<()> {
        let result = sqlx::query(
            "DELETE FROM Post WHERE id = ?;")
            .bind(post_id)
            .execute(&self.conn_pool)
            .await;
        match result {
            Ok(res) => expected_rows_affected(res, 1),
            Err(e) => Err(log_error(DBError::from(e)))
        }
    }

    #[cfg(test)]
    pub async fn delete_comment(&self, comment_id: u64) -> DBResult<()> {
        let result = sqlx::query(
            "DELETE FROM Comment WHERE id = ?;")
            .bind(comment_id)
            .execute(&self.conn_pool)
            .await;
        match result {
            Ok(res) => expected_rows_affected(res, 1),
            Err(e) => Err(log_error(DBError::from(e)))
        }
    }

    pub async fn delete_post_like(&self, post_id: u64, account_id: u64) -> DBResult<()> {
        let result = sqlx::query(
            "DELETE FROM PostLike
            WHERE post_id = ?
            AND account_id = ?;")
            .bind(post_id)
            .bind(account_id)
            .execute(&self.conn_pool)
            .await;
        match result {
            Ok(res) => expected_rows_affected(res, 1),
            Err(e) => Err(log_error(DBError::from(e)))
        }
    }

    pub async fn delete_comment_like(&self, comment_id: u64, account_id: u64) -> DBResult<()> {
        let result = sqlx::query(
            "DELETE FROM CommentLike
            WHERE comment_id = ?
            AND account_id = ?;")
            .bind(comment_id)
            .bind(account_id)
            .execute(&self.conn_pool)
            .await;
        match result {
            Ok(res) => expected_rows_affected(res, 1),
            Err(e) => Err(log_error(DBError::from(e)))
        }
    }

    #[cfg(test)]
    async fn delete_comment_by_id_and_body(&self, id: u64, body: &str) -> DBResult<()> {
        let result = sqlx::query(
            "DELETE FROM Comment
            WHERE commenter_id = ?
            AND body = ?")
            .bind(id)
            .bind(body)
            .execute(&self.conn_pool)
            .await;
        match result {
            Ok(_)  => Ok(()),
            Err(e) => Err(DBError::from(e))
        }
    }

    #[cfg(test)]
    async fn delete_post_by_title_and_body(&self, title: &str, body: &str) -> DBResult<()> {
        let result = sqlx::query(
            "DELETE FROM Post
            WHERE title = ?
            AND body = ?")
            .bind(title)
            .bind(body)
            .execute(&self.conn_pool)
            .await;
        match result {
            Ok(_)  => Ok(()),
            Err(e) => Err(DBError::from(e)),
        }
    }
}

fn expected_rows_affected(result: MySqlQueryResult, expected_rows: u64) -> DBResult<()> {
    if result.rows_affected() == expected_rows {
        Ok(())
    } else {
        Err(log_error(DBError::UnexpectedRowsAffected {
            expected: expected_rows, actual: result.rows_affected()
        }))
    }
}

fn log_error(err: DBError) -> DBError {
    warn!("{}", err);
    err
}

#[cfg(test)]
mod test {
    use std::mem::discriminant;
    use std::mem::Discriminant;
    use crate::models::Comment;
    use crate::models::MySqlBool;
    use crate::models::NewComment;
    use crate::models::NewPost;
    use crate::models::Post;

    use super::Database;
    use super::DBError;
    use dotenv;
    
    const DB_ERR_URA: Discriminant<DBError> = discriminant(&DBError::UnexpectedRowsAffected {
        expected: 0, actual: 0
    });
    const DB_ERR_NR: Discriminant<DBError> = discriminant(&DBError::NoResult);
    const DB_ERR_SQLX: Discriminant<DBError> = discriminant(&DBError::SQLXError(sqlx::Error::PoolClosed));

    async fn test_context() -> Database {
        dotenv::dotenv().ok();
        let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is not set");
        Database::new(&db_url).await
    }

    // The below test(s) require that the MySql database is not empty. At minimum, the
    // `devtest_data.sql` should be used.

    #[actix_web::test]
    async fn test_errors() {
        let db: Database = test_context().await;

        // CRUD

        // Create
        let post_invalid_poster_id = NewPost {
            poster_id: 0,
            title: "bad_posted_id".to_string(),
            body: "bad_posted_id".to_string(),
        };
        assert_eq!(DB_ERR_SQLX, discriminant(&db.create_post(post_invalid_poster_id).await.unwrap_err()));

        let comment_on_invalid_post_id = NewComment {
            post_id: 0,  // all ids start from 1
            commenter_id: 1,
            comment_reply_id: None,
            body: "".into()
        };

        assert_eq!(DB_ERR_SQLX, discriminant(&db.create_comment(comment_on_invalid_post_id).await.unwrap_err()));

        let comment_by_invalid_commenter_id = NewComment {
            post_id: 1,
            commenter_id: 0, // all ids start from 1
            comment_reply_id: None,
            body: "".into()
        };
        assert_eq!(DB_ERR_SQLX, discriminant(&db.create_comment(comment_by_invalid_commenter_id).await.unwrap_err()));

        // Invalid post_id
        assert_eq!(DB_ERR_URA, discriminant(&db.create_post_like(0, 1).await.unwrap_err()));
        // Invalid account_id
        assert_eq!(DB_ERR_URA, discriminant(&db.create_post_like(1, 0).await.unwrap_err()));

        // Invalid comment_id
        assert_eq!(DB_ERR_URA, discriminant(&db.create_comment_like(0, 1).await.unwrap_err()));
        // Invalid account_id
        assert_eq!(DB_ERR_URA, discriminant(&db.create_comment_like(1, 0).await.unwrap_err()));
        
        // Read
        assert_eq!(DB_ERR_NR, discriminant(&db.read_post_by_id(0).await.unwrap_err()));
        // read_posts_by_user, read_comments_by_user, and read_comments_of_post will return an empty
        // vec with an invalid post or account id value.

        // Update
        assert_eq!(DB_ERR_URA, discriminant(&db.update_account_password(0, "", "").await.unwrap_err()));
        assert_eq!(DB_ERR_URA, discriminant(&db.update_post_body(0, "".to_string()).await.unwrap_err()));
        assert_eq!(DB_ERR_URA, discriminant(&db.update_comment_body(0, "".to_string()).await.unwrap_err()));
    
        // Delete
        assert_eq!(DB_ERR_URA, discriminant(&db.delete_post(0).await.unwrap_err()));
        assert_eq!(DB_ERR_URA, discriminant(&db.delete_post_like(0, 0).await.unwrap_err()));
        assert_eq!(DB_ERR_URA, discriminant(&db.delete_comment(0).await.unwrap_err()));
        assert_eq!(DB_ERR_URA, discriminant(&db.delete_comment_like(0, 0).await.unwrap_err()));
    }

    #[actix_web::test]
    async fn test_post_operations() {
        let db: Database = test_context().await;

        const POSTER_ID: u64 = 1;  // 1 = devtest_1
        const TITLE: &str = "#@!test_post_operations";
        const FIRST_BODY: &str = "test post body";
        const SECOND_BODY: &str = "updated/edited test post body";

        let predicate = |p: &Post| p.poster_id.eq(&POSTER_ID) && p.title.eq(TITLE);
  
        // clear any left-over posts from previous failed test runs
        assert_eq!(Ok(()), db.delete_post_by_title_and_body(TITLE, FIRST_BODY).await, "failed to setup 1");
        assert_eq!(Ok(()), db.delete_post_by_title_and_body(TITLE, SECOND_BODY).await, "failed to setup 2");
        
        // Ensure test post is not present
        let before_posting = db.read_posts_by_user(POSTER_ID).await.unwrap();
        assert_eq!(0, before_posting.iter().filter(|p| predicate(p)).count());
        
        // Create, add, and check that the test post was added
        let new_post = NewPost {
            poster_id: POSTER_ID,
            title: TITLE.to_string(),
            body: FIRST_BODY.to_string()
        };
        assert_eq!(Ok(()), db.create_post(new_post).await);
        let after_posting = db.read_posts_by_user(POSTER_ID).await.unwrap();
        assert_eq!(1, after_posting.iter().filter(|p| predicate(p)).count());
        let retrieved_post_before_edit = after_posting.iter().find(|p| predicate(p)).unwrap();
        
        assert_eq!(POSTER_ID, retrieved_post_before_edit.poster_id);
        assert_eq!(TITLE, retrieved_post_before_edit.title);
        assert_eq!(FIRST_BODY, retrieved_post_before_edit.body);
        assert_eq!(0, retrieved_post_before_edit.likes);
        assert_eq!(MySqlBool(false), retrieved_post_before_edit.edited);

        let test_post_id = retrieved_post_before_edit.id;

        // Edit the test post and re-check
        assert_eq!(Ok(()), db.update_post_body(test_post_id, SECOND_BODY.into()).await);
        let retrieved_post_after_edit = db.read_post_by_id(test_post_id).await.unwrap();

        assert_eq!(POSTER_ID, retrieved_post_after_edit.poster_id);
        assert_eq!(TITLE, retrieved_post_after_edit.title);
        assert_eq!(SECOND_BODY, retrieved_post_after_edit.body);
        assert_eq!(0, retrieved_post_after_edit.likes);
        assert_eq!(MySqlBool(true), retrieved_post_after_edit.edited);

        // Delete the test post and check that it cannot be read
        assert_eq!(Ok(()), db.delete_post(test_post_id).await);
        let after_delete = db.read_post_by_id(test_post_id).await;
        assert_eq!(true, after_delete.is_err());
        assert_eq!(DB_ERR_NR, discriminant(&after_delete.unwrap_err()));
    }

    #[actix_web::test]
    async fn test_comment_operations() {
        const POST_ID: u64 = 1;
        const COMMENTER_ID_ONE: u64 = 1;
        const COMMENTER_ID_TWO: u64 = 2;
        const FIRST_BODY: &str = "#@!test_comment_operations";
        const SECOND_BODY: &str = "#@!test_comment_operations updated/edited";

        let db: Database = test_context().await;

        let predicate = |c: &Comment| {
            (c.commenter_id == COMMENTER_ID_ONE || c.commenter_id == COMMENTER_ID_TWO)
            && (c.body.eq(FIRST_BODY) || c.body.eq(SECOND_BODY))
        };

        // Clear any left-over test comments and create
        assert_eq!(Ok(()), db.delete_comment_by_id_and_body(COMMENTER_ID_ONE, FIRST_BODY).await);
        assert_eq!(Ok(()), db.delete_comment_by_id_and_body(COMMENTER_ID_TWO, FIRST_BODY).await);
        assert_eq!(Ok(()), db.delete_comment_by_id_and_body(COMMENTER_ID_ONE, SECOND_BODY).await);
        assert_eq!(Ok(()), db.delete_comment_by_id_and_body(COMMENTER_ID_TWO, SECOND_BODY).await);

        // Ensure test comments are not present
        let before_comment_one = db.read_comments_of_post(POST_ID).await.unwrap();
        assert_eq!(false, before_comment_one.iter().any(|c| predicate(c)));

        // Create, add and check first test comment
        let first_comment = NewComment {
            post_id: POST_ID,
            commenter_id: COMMENTER_ID_ONE,
            comment_reply_id: None,
            body: FIRST_BODY.to_string()
        };

        assert_eq!(Ok(()), db.create_comment(first_comment).await);
        let after_comment_one = db.read_comments_of_post(POST_ID).await.unwrap();
        assert_eq!(1, after_comment_one.iter().filter(|c| predicate(c)).count());
        let retrieved_comment_one = after_comment_one.iter().find(|c| predicate(c)).unwrap();

        assert_eq!(POST_ID, retrieved_comment_one.post_id);
        assert_eq!(COMMENTER_ID_ONE, retrieved_comment_one.commenter_id);
        assert_eq!(FIRST_BODY, retrieved_comment_one.body);
        assert_eq!(None, retrieved_comment_one.comment_reply_id);
        assert_eq!(0, retrieved_comment_one.likes);
        assert_eq!(MySqlBool(false), retrieved_comment_one.edited);

        let comment_one_id = retrieved_comment_one.id;

        // Update/edit first test comment and check
        assert_eq!(Ok(()), db.update_comment_body(comment_one_id, SECOND_BODY.into()).await);
        let after_comment_one_edit = db.read_comments_of_post(POST_ID).await.unwrap();
        assert_eq!(1, after_comment_one.iter().filter(|c| predicate(c)).count());
        let retrieved_comment_one_edited = after_comment_one_edit.iter().find(|c| predicate(c)).unwrap();

        assert_eq!(POST_ID, retrieved_comment_one_edited.post_id);
        assert_eq!(COMMENTER_ID_ONE, retrieved_comment_one_edited.commenter_id);
        assert_eq!(SECOND_BODY, retrieved_comment_one_edited.body);
        assert_eq!(None, retrieved_comment_one_edited.comment_reply_id);
        assert_eq!(0, retrieved_comment_one_edited.likes);
        assert_eq!(MySqlBool(true), retrieved_comment_one_edited.edited);

        // Create, add, and check second test comment
        let comment_two = NewComment {
            post_id: POST_ID,
            commenter_id: COMMENTER_ID_TWO,
            comment_reply_id: Some(comment_one_id),
            body: FIRST_BODY.to_string()
        };

        assert_eq!(Ok(()), db.create_comment(comment_two).await);
        let after_comment_two = db.read_comments_of_post(POST_ID).await.unwrap();
        assert_eq!(2, after_comment_two.iter().filter(|c| predicate(c)).count());
        assert_eq!(1, after_comment_two
            .iter()
            .filter(|c| predicate(c) && c.comment_reply_id.is_some_and(|id| id == comment_one_id))
            .count()
        );
        let retrieved_comment_two = after_comment_two
            .iter()
            .find(|c| predicate(c) && c.comment_reply_id.is_some_and(|id| id == comment_one_id))
            .unwrap();

        assert_eq!(POST_ID, retrieved_comment_two.post_id);
        assert_eq!(COMMENTER_ID_TWO, retrieved_comment_two.commenter_id);
        assert_eq!(FIRST_BODY, retrieved_comment_two.body);
        assert_eq!(Some(comment_one_id), retrieved_comment_two.comment_reply_id);
        assert_eq!(0, retrieved_comment_two.likes);
        assert_eq!(MySqlBool(false), retrieved_comment_two.edited);

        let comment_two_id = retrieved_comment_two.id;

        // set first test comment as "[DELETED]", where second test comment is a reply to it
        assert_eq!(Ok(()), db.update_comment_body(comment_one_id, "[DELETED]".to_string()).await);
        let comments_after_delete = db.read_comments_of_post(POST_ID).await.unwrap();
        let comment_one_deleted = comments_after_delete
            .iter()
            .find(|c| c.id.eq(&comment_one_id));
        assert_eq!(true, comment_one_deleted.is_some());
        let comment_one_deleted = comment_one_deleted.unwrap();
        assert_eq!(POST_ID, comment_one_deleted.post_id);
        assert_eq!(COMMENTER_ID_ONE, comment_one_deleted.commenter_id);
        assert_eq!("[DELETED]", comment_one_deleted.body);
        assert_eq!(None, comment_one_deleted.comment_reply_id);
        assert_eq!(0, comment_one_deleted.likes);
        assert_eq!(MySqlBool(true), comment_one_deleted.edited);

        // Actually delete test comments
        assert_eq!(Ok(()), db.delete_comment(comment_two_id.clone()).await);  // reply first (fk)
        assert_eq!(Ok(()), db.delete_comment(comment_one_id.clone()).await);
        assert_eq!(0, db.read_comments_of_post(POST_ID).await
            .unwrap()
            .iter()
            .filter(|c| c.id.eq(&comment_one_id) || c.id.eq(&comment_two_id))
            .count()
        );
    }

}
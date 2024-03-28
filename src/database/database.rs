use log::warn;
use sqlx::{MySql, Pool, Row};
use sqlx::mysql::{MySqlPoolOptions, MySqlQueryResult};

use crate::models::{Account, AccountFromDB, Comment, Post};
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


    pub async fn create_post(&self, post: Post) -> DBResult<()> {
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

    pub async fn create_comment(&self, comment: Comment) -> DBResult<()> {
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

    pub async fn read_account_by_id(&self, id: u64) -> DBResult<AccountFromDB> {
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

    pub async fn read_account_id(&self, details: Account) -> DBResult<u64> {
        let result = sqlx::query(
            "SELECT id
            FROM Account
            WHERE username = ?
            AND password_hash = ?
            LIMIT 1;")
            .bind(details.username)
            .bind(details.password)
            .fetch_one(&self.conn_pool)
            .await;
        
        match result {
            Ok(id) => Ok(id.try_get(0)?),
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
    use crate::models::Post;

    use super::Database;
    use super::DBError;
    use dotenv;
    
    const DB_ERR_URA: Discriminant<DBError> = discriminant(&DBError::UnexpectedRowsAffected {
        expected: 0, actual: 0
    });
    const DB_ERR_NR: Discriminant<DBError> = discriminant(&DBError::NoResult);
    const DB_ERR_SQLX: Discriminant<DBError> = discriminant(&DBError::SQLXError(sqlx::Error::PoolClosed));

    // The below test(s) require that the MySql database is not empty. At minimum, the
    // `devtest_data.sql` should be used.

    #[actix_web::test]
    async fn test_errors() {
        dotenv::dotenv().ok();
        let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is not set");
        let db: Database = Database::new(&db_url).await;

        // CRUD
        // Create
        let post_invalid_poster_id = Post {
            id: None,
            poster_id: 0,
            title: "bad_posted_id".to_string(),
            body: "bad_posted_id".to_string(),
            likes: None,
            time_stamp: None,
            edited: None
        };
        assert_eq!(DB_ERR_SQLX, discriminant(&db.create_post(post_invalid_poster_id).await.unwrap_err()));

        let comment_invalid_post_id = Comment {
            id: None,
            post_id: 0,
            commenter_id: 1,
            body: "".into(),
            comment_reply_id: None,
            likes: None,
            time_stamp: None,
            edited: None
        };
        assert_eq!(DB_ERR_SQLX, discriminant(&db.create_comment(comment_invalid_post_id).await.unwrap_err()));

        let comment_invalid_commenter_id = Comment {
            id: None,
            post_id: 1,
            commenter_id: 0,
            body: "".into(),
            comment_reply_id: None,
            likes: None,
            time_stamp: None,
            edited: None
        };
        assert_eq!(DB_ERR_SQLX, discriminant(&db.create_comment(comment_invalid_commenter_id).await.unwrap_err()));

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
}
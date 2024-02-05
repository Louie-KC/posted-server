use log::warn;
use sqlx::{MySql, Pool, Row};
use sqlx::mysql::{MySqlPoolOptions, MySqlQueryResult};

use crate::models::{Account, Comment, Post};
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

    pub async fn read_account_id(&self, details: Account) -> DBResult<u64> {
        let result = sqlx::query(
            "SELECT id
            FROM Account
            WHERE username = ?
            AND password_hash = ?
            LIMIT 1;")
            .bind(details.username)
            .bind(details.password_hash)
            .fetch_one(&self.conn_pool)
            .await;
        
        match result {
            Ok(id) => Ok(id.try_get(0)?),
            Err(e) => Err(log_error(DBError::from(e)))
        }
    }

    pub async fn read_posts(&self, max_posts: u64) -> DBResult<Vec<Post>> {
        let result = sqlx::query_as::<_, Post>(
            "SELECT p.*, CAST(count(pl.account_id) AS UNSIGNED) AS 'likes'
            FROM Post p
            LEFT JOIN PostLike pl
            ON p.id = pl.post_id
            GROUP BY p.id
            LIMIT ?;")
            .bind(max_posts)
            .fetch_all(&self.conn_pool)
            .await;
        match result {
            Ok(posts) => Ok(posts),
            Err(e)  => Err(log_error(DBError::from(e)))
        }
    }

    pub async fn read_post_by_id(&self, post_id: u64) -> DBResult<Post> {
        let result = sqlx::query_as::<_, Post>(
            "SELECT p.*, CAST(COUNT(pl.account_id) AS UNSIGNED) AS 'likes'
            FROM Post p
            LEFT JOIN PostLike pl
            ON p.id = pl.post_id
            WHERE p.id = ?
            GROUP BY p.id;")
            .bind(post_id)
            .fetch_one(&self.conn_pool)
            .await;
        match result {
            Ok(post) => Ok(post),
            Err(e) => Err(DBError::from(e))
        }
    }

    pub async fn read_posts_by_user(&self, user_id: u64) -> DBResult<Vec<Post>> {
        let result = sqlx::query_as::<_, Post>(
            "SELECT p.*, CAST(count(pl.account_id) AS UNSIGNED) AS 'likes'
            FROM Post p
            LEFT JOIN PostLike pl
            ON p.id = pl.post_id
            WHERE p.poster_id = ?
            GROUP BY p.id;")
            .bind(user_id)
            .fetch_all(&self.conn_pool)
            .await;
        match result {
            Ok(posts) => Ok(posts),
            Err(e) => Err(log_error(DBError::from(e)))
        }
    }

    pub async fn read_comments_of_post(&self, post_id: u64) -> DBResult<Vec<Comment>> {
        let result = sqlx::query_as::<_, Comment>(
            "SELECT c.*, CAST(count(cl.comment_id) AS UNSIGNED) AS 'likes'
            FROM Comment c
            LEFT JOIN CommentLike cl
            ON c.id = cl.comment_id
            WHERE c.post_id = ?
            GROUP BY c.id")
            .bind(post_id)
            .fetch_all(&self.conn_pool)
            .await;

        match result {
            Ok(comments) => Ok(comments),
            Err(e) => Err(log_error(DBError::from(e)))
        }
    }

    pub async fn read_comments_by_user(&self, user_id: u64) -> DBResult<Vec<Comment>> {
        let result = sqlx::query_as::<_, Comment>(
            "SELECT c.*, CAST(count(cl.comment_id) AS UNSIGNED) AS 'likes'
            FROM Comment c
            LEFT JOIN CommentLike cl
            ON c.id = cl.comment_id
            WHERE c.commenter_id = ?
            GROUP BY c.id")
            .bind(user_id)
            .fetch_all(&self.conn_pool)
            .await;

        match result {
            Ok(comments) => Ok(comments),
            Err(e) => Err(log_error(DBError::from(e)))
        }
    }

    pub async fn read_post_likes(&self, post_id: u64) -> DBResult<u64> {
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

    pub async fn read_comment_likes(&self, comment_id: u64) -> DBResult<u64> {
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
        Err(log_error(DBError::UnexpectedRowsAffected(expected_rows, result.rows_affected())))
    }
}

fn log_error(err: DBError) -> DBError {
    warn!("{}", err);
    err
}
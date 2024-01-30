mod database;
mod models;

use std::env;
use dotenv::dotenv;
use crate::database::database::Database;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL is not set");
    let database = Database::new(&db_url).await;

    match database.read_posts(5).await {
        Ok(posts) => posts.iter().for_each(|post| println!("{:?}", post)),
        Err(e) => println!("read_posts error: {:?}", e)
    };

    match database.read_post_likes(1).await {
        Ok(likes) => println!("likes: {}", likes),
        Err(e) => println!("read_post_likes error: {:?}", e)
    }
    
    Ok(())
}

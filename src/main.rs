mod api;
mod auth;
mod cache;
mod database;
mod models;

// use std::sync::Mutex;

use dotenv::dotenv;
use actix_web::{App, HttpServer, web, middleware::Logger};

use crate::cache::cache::Cache;
use crate::database::database::Database;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "info");

    dotenv().ok();
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is not set");
    let database = Database::new(&db_url).await;
    let db_data = web::Data::new(database);

    // let auth_service: Mutex<AuthService> = AuthService::new().into();
    // let auth_data = web::Data::new(auth_service);

    let cache_url = std::env::var("REDIS_DATABASE_URL").expect("REDIS_DATABASE_URL is not set");
    let cache = Cache::new(&cache_url).await;
    let cache_data = web::Data::new(cache);

    let server_addr = "0.0.0.0";
    let server_port = 8080;
    let app = HttpServer::new(move ||
        App::new()
            .wrap(Logger::new("%a \"%r\" %s %bb %Tsec"))
            .app_data(db_data.clone())
            // .app_data(auth_data.clone())
            .app_data(cache_data.clone())
            .configure(api::api::config)
    )
    .workers(4)
    .bind((server_addr, server_port))?;

    println!("Server running at http://{}:{}/", server_addr, server_port);
    env_logger::init();

    app.run().await
}

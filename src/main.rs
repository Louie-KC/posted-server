mod api;
mod auth;
mod cache;
mod database;
mod models;

use std::sync::Mutex;

use actix_web::{App, HttpServer, web, middleware::Logger};
use argon2::Argon2;
use dotenv::dotenv;

use crate::auth::auth::AuthService;
use crate::database::database::Database;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "info");

    dotenv().ok();
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is not set");
    let database = Database::new(&db_url).await;
    let db_data = web::Data::new(database);

    let redis_url = std::env::var("REDIS_DATABASE_URL").expect("REDIS_DATABASE_URL is not set");
    let auth_service = AuthService::new(&redis_url);
    let auth_service_data = web::Data::new(Mutex::new(auth_service));

    let server_addr = "0.0.0.0";
    let server_port = 8080;

    let argon2_encrypt = Argon2::default();
    let encrypt_data = web::Data::new(argon2_encrypt);

    let app = HttpServer::new(move ||
        App::new()
            .wrap(Logger::new("%a \"%r\" %s %bb %Tsec"))
            .app_data(db_data.clone())
            .app_data(auth_service_data.clone())
            .app_data(encrypt_data.clone())
            .configure(api::api::config)
    )
    .workers(1)
    .bind((server_addr, server_port))?;

    println!("Server running at http://{}:{}/", server_addr, server_port);
    env_logger::init();

    app.run().await
}

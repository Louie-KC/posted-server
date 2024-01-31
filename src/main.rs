mod api;
mod database;
mod models;

use std::env;
use dotenv::dotenv;
use actix_web::{App, HttpServer, web};

use crate::database::database::Database;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL is not set");
    let database = Database::new(&db_url).await;

    let data = web::Data::new(database);

    let server_addr = "0.0.0.0";
    let server_port = 8080;
    let app = HttpServer::new(move ||
        App::new()
            .app_data(data.clone())
            .configure(api::api::config)
    )
    .bind((server_addr, server_port))?
    .run();

    println!("Server running at http://{}:{}/", server_addr, server_port);

    app.await
}

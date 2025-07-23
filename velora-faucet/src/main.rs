use actix_web::{web, App, HttpServer};
use std::env;
use dotenv::dotenv;
use log::info;

mod handlers;
mod state;
mod utils;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let server_host = env::var("FAUCET_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let server_port = env::var("FAUCET_PORT").unwrap_or_else(|_| "8080".to_string()).parse::<u16>().expect("Invalid port");

    info!("Starting Velora Faucet at http://{}:{}", server_host, server_port);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(state::AppState::new()))
            .route("/faucet", web::post().to(handlers::faucet_request))
    })
    .bind((server_host, server_port))?
    .run()
    .await
}

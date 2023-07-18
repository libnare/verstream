use std::env;
use std::net::IpAddr;
use std::str::FromStr;

use actix_web::{App, get, HttpResponse, HttpServer, middleware, Responder, web};
use actix_web::http::StatusCode;
use actix_web::web::Data;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::Client;
use env_logger::Env;
use futures_util::{Stream, TryStreamExt};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
struct Opt {
    bucket: String,
    object: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Err {
    code: u16,
    msg: String,
}

fn check_env_var(var_name: &str) {
    if env::var(var_name).is_err() {
        panic!("{} is not set", var_name);
    }
}

fn get_bind_address() -> IpAddr {
    let bind_address = env::var("SERVER_ADDRESS")
        .unwrap_or_else(|_| String::from("127.0.0.1"));

    IpAddr::from_str(&bind_address)
        .unwrap_or_else(|_| IpAddr::from_str("127.0.0.1").unwrap())
}

async fn get_object(client: &Client, opt: Opt) -> Result<impl Stream<Item=Result<bytes::Bytes, anyhow::Error>>, anyhow::Error> {
    let object = client
        .get_object()
        .bucket(opt.bucket)
        .key(opt.object)
        .send()
        .await?;

    Ok(object.body.map_err(Into::into))
}

#[get("/{tail:.*}")]
async fn serve(key: web::Path<String>, client: Data<Client>) -> impl Responder {
    let options = Opt {
        bucket: env::var("AWS_BUCKET").expect("AWS_BUCKET is not set"),
        object: key.clone(),
    };

    match get_object(&client, options).await {
        Ok(stream) => {
            let mut response = HttpResponse::Ok();
            if let Ok(value) = env::var("HEADER_CC_1Y") {
                if value.eq_ignore_ascii_case("true") {
                    response.insert_header(("Cache-Control", "public, max-age=31536000"));
                }
            }
            response.streaming(stream)
        },
        Err(err) => {
            let root_cause = err.root_cause();
            log::error!("Error retrieving object: {}", root_cause);

            let status_code = if root_cause.to_string().contains("NoSuchKey") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };

            HttpResponse::build(status_code)
                .json(Err {
                    code: status_code.as_u16(),
                    msg: format!("{}", root_cause),
                })
        }
    }
}

#[get("/")]
async fn index() -> impl Responder {
    HttpResponse::Ok().body("Server is running")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    let env_vars = &["AWS_ENDPOINT", "AWS_BUCKET"];

    for var in env_vars {
        check_env_var(var);
    }

    let aws_access_key_exists = check_env_var("AWS_ACCESS_KEY_ID");
    let aws_secret_key_exists = check_env_var("AWS_SECRET_ACCESS_KEY");

    if aws_access_key_exists != aws_secret_key_exists {
        panic!("Either both AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY must be set or neither.");
    }

    let endpoint = env::var("AWS_ENDPOINT").ok();

    let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");

    let shared_config = aws_config::from_env()
        .region(region_provider)
        .endpoint_url(endpoint.unwrap_or_default())
        .load()
        .await;
    let client = Client::new(&shared_config);

    let bind_address = get_bind_address();
    let bind_port = env::var("BIND_PORT")
        .unwrap_or_else(|_| String::from("8080"))
        .parse::<u16>()
        .expect("Invalid port number");

    let server = HttpServer::new(move || {
        App::new()
            .app_data(Data::new(client.clone()))
            .service(index)
            .service(serve)
            .wrap(middleware::Logger::default())
    })
        .bind(format!("{}:{}", bind_address, bind_port))?;

    log::info!("Starting server on {}:{}", bind_address, bind_port);
    server.run().await
}

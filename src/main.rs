use actix_web::{delete, get, put, web, App, HttpResponse, HttpServer, Responder};
use log::{error, info};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::env;
use std::sync::Mutex;

#[derive(Serialize, Deserialize)]
struct KeyValue {
    value: serde_json::Value,
}

struct AppState {
    store: Mutex<HashMap<String, serde_json::Value>>,
    forwarding_address: Option<String>,
}

#[put("/kvs/{key}")]
async fn put(
    key: web::Path<String>,
    item: Option<web::Json<KeyValue>>,
    data: web::Data<AppState>,
) -> impl Responder {
    let key_str = key.into_inner();
    log::info!("PUT request for key: {}", key_str);

    if let Some(address) = &data.forwarding_address {
        let forwarding_url = format!("http://{}/kvs/{}", address, key_str);
        let client = Client::new();

        match item {
            Some(item_val) => {
                let res = client.put(&forwarding_url).json(&item_val).send().await;

                match res {
                    Ok(ok_res) => {
                        let status = ok_res.status();
                        let res_json: serde_json::Value = match ok_res.json().await {
                            Ok(json) => json,
                            Err(e) => return HttpResponse::InternalServerError().json(
                                json!({ "error": format!("Failed to parse JSON response: {}", e) }),
                            ),
                        };

                        return HttpResponse::build(status).json(res_json);
                    }
                    Err(_) => {
                        return HttpResponse::ServiceUnavailable()
                            .json(json!({ "error": "Cannot forward request" }))
                    }
                }
            }
            None => {
                return HttpResponse::BadRequest()
                    .json(json!({ "error": "PUT request does not specify a value" }))
            }
        }
    }

    if key_str.len() > 50 {
        return HttpResponse::BadRequest().json(json!({ "error": "Key is too long" }));
    }

    if item.is_none() {
        return HttpResponse::BadRequest()
            .json(json!({ "error": "PUT request does not specify a value" }));
    }

    let value = item.unwrap().value.clone();

    let mut store = data.store.lock().unwrap();
    log::debug!("Store before PUT: {:?}", store);
    let result = if store.contains_key(&key_str) {
        store.insert(key_str.clone(), value.clone());
        HttpResponse::Ok().json(json!({ "result": "replaced" }))
    } else {
        store.insert(key_str.clone(), value.clone());
        HttpResponse::Created().json(json!({ "result": "created" }))
    };

    log::debug!("Store after PUT: {:?}", store);
    result
}

#[get("/kvs/{key}")]
async fn get(key: web::Path<String>, data: web::Data<AppState>) -> impl Responder {
    let key_str = key.into_inner();
    info!("GET request for key: {}", key_str);

    if let Some(address) = &data.forwarding_address {
        let forwarding_url = format!("http://{}/kvs/{}", address, key_str);
        let client = Client::new();
        let res = client.get(&forwarding_url).send().await;
        match res {
            Ok(ok_res) => {
                let status = ok_res.status();
                let res_json: serde_json::Value = match ok_res.json().await {
                    Ok(json) => json,
                    Err(e) => {
                        return HttpResponse::InternalServerError().json(
                            json!({ "error": format!("Failed to parse JSON response: {}", e) }),
                        )
                    }
                };
                return HttpResponse::build(status).json(res_json);
            }
            Err(e) => {
                error!("Error forwarding request to {}: {}", address, e);
                return HttpResponse::ServiceUnavailable()
                    .json(json!({ "error": "Cannot forward request" }));
            }
        }
    }

    let store = data.store.lock().unwrap();
    if let Some(value) = store.get(&key_str) {
        HttpResponse::Ok().json(json!({ "result": "found", "value": value }))
    } else {
        HttpResponse::NotFound().json(json!({ "error": "Key does not exist" }))
    }
}

#[delete("/kvs/{key}")]
async fn delete(key: web::Path<String>, data: web::Data<AppState>) -> impl Responder {
    let key_str = key.into_inner();
    info!("DELETE request for key: {}", key_str);

    if let Some(address) = &data.forwarding_address {
        let forwarding_url = format!("http://{}/kvs/{}", address, key_str);
        let client = Client::new();
        let res = client.delete(&forwarding_url).send().await;
        match res {
            Ok(ok_res) => {
                let status = ok_res.status();
                let res_json: serde_json::Value = match ok_res.json().await {
                    Ok(json) => json,
                    Err(e) => {
                        return HttpResponse::InternalServerError().json(
                            json!({ "error": format!("Failed to parse JSON response: {}", e) }),
                        )
                    }
                };
                return HttpResponse::build(status).json(res_json);
            }
            Err(e) => {
                error!("Error forwarding request to {}: {}", address, e);
                return HttpResponse::ServiceUnavailable()
                    .json(json!({ "error": "Cannot forward request" }));
            }
        }
    }

    let mut store = data.store.lock().unwrap();
    if store.remove(&key_str).is_some() {
        HttpResponse::Ok().json(json!({ "result": "deleted" }))
    } else {
        HttpResponse::NotFound().json(json!({ "error": "Key does not exist" }))
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let forwarding_address = env::var("FORWARDING_ADDRESS").ok();

    let app_data = web::Data::new(AppState {
        store: Mutex::new(HashMap::new()),
        forwarding_address,
    });

    HttpServer::new(move || {
        App::new()
            .app_data(app_data.clone())
            .service(put)
            .service(get)
            .service(delete)
    })
    .bind("0.0.0.0:8090")?
    .run()
    .await
}

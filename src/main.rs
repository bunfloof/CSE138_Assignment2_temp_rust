use actix_web::{delete, get, put, web, App, HttpResponse, HttpServer, Responder};
use log::{debug, info};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Serialize, Deserialize)]
struct KeyValue {
    value: serde_json::Value,
}

struct AppState {
    store: Mutex<HashMap<String, serde_json::Value>>,
}

#[put("/kvs/{key}")]
async fn put(
    key: web::Path<String>,
    item: Option<web::Json<KeyValue>>,
    data: web::Data<AppState>,
) -> impl Responder {
    let key_str = key.into_inner();
    log::info!("PUT request for key: {}", key_str);

    if key_str.len() > 50 {
        return HttpResponse::BadRequest().json(json!({ "error": "Key is too long" }));
    }

    let value = match item {
        Some(v) => v.value.clone(),
        None => {
            return HttpResponse::BadRequest()
                .json(json!({ "error": "PUT request does not specify a value" }))
        }
    };

    let mut store = data.store.lock().unwrap();
    debug!("Store before PUT: {:?}", store);
    let result = if store.contains_key(&key_str) {
        store.insert(key_str.clone(), value);
        HttpResponse::Ok().json(json!({ "result": "replaced" }))
    } else {
        store.insert(key_str.clone(), value);
        HttpResponse::Created().json(json!({ "result": "created" }))
    };
    debug!("Store after PUT: {:?}", store);
    result
}

#[get("/kvs/{key}")]
async fn get(key: web::Path<String>, data: web::Data<AppState>) -> impl Responder {
    let key_str = key.into_inner();
    log::info!("GET request for key: {}", key_str);

    let store = data.store.lock().unwrap();
    debug!("Store during GET: {:?}", store);
    if let Some(value) = store.get(&key_str) {
        HttpResponse::Ok().json(json!({ "result": "found", "value": value }))
    } else {
        HttpResponse::NotFound().json(json!({ "error": "Key does not exist" }))
    }
}

#[delete("/kvs/{key}")]
async fn delete(key: web::Path<String>, data: web::Data<AppState>) -> impl Responder {
    let key_str = key.into_inner();
    log::info!("DELETE request for key: {}", key_str);

    let mut store = data.store.lock().unwrap();
    debug!("Store before DELETE: {:?}", store);
    if store.remove(&key_str).is_some() {
        HttpResponse::Ok().json(json!({ "result": "deleted" }))
    } else {
        HttpResponse::NotFound().json(json!({ "error": "Key does not exist" }))
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let app_data = web::Data::new(AppState {
        store: Mutex::new(HashMap::new()),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(app_data.clone())
            .service(put)
            .service(get)
            .service(delete)
    })
    .bind("127.0.0.1:8090")?
    .run()
    .await
}

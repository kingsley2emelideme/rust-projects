mod quant;
mod volatility;

use actix_web::{App, HttpResponse, HttpServer, Responder, get, post, web};
use colored::*;
use quant::{FullMetricsResponse, IvPayload, OptionInputs};
use std::time::Instant;

#[post("/calculate")]
async fn calculate_metrics(payload: web::Json<OptionInputs>) -> impl Responder {
    let start_time = Instant::now();
    let inputs = payload.into_inner();

    let (analytical_call, analytical_put, greeks) = match inputs.calculate_greeks() {
        Ok(res) => res,
        Err(e) => return HttpResponse::BadRequest().body(format!("Validation Error: {}", e)),
    };

    let (mc_call, mc_put) = match inputs.monte_carlo_price(1_000_000) {
        Ok(res) => res,
        Err(e) => return HttpResponse::InternalServerError().body(format!("Math Error: {}", e)),
    };

    let duration = start_time.elapsed().as_secs_f64() * 1000.0;

    HttpResponse::Ok().json(FullMetricsResponse {
        analytical_call,
        analytical_put,
        monte_carlo_call: mc_call,
        monte_carlo_put: mc_put,
        greeks,
        processing_time_ms: duration,
    })
}

// NEW ROUTE: COMPUTE IMPLIED VOLATILITY VIA POST REQUEST
#[post("/calculate-iv")]
async fn calculate_iv_endpoint(payload: web::Json<IvPayload>) -> impl Responder {
    let body = payload.into_inner();
    match body
        .inputs
        .calculate_implied_volatility(body.observed_market_price, body.is_call)
    {
        Ok(iv) => HttpResponse::Ok().json(serde_json::json!({ "implied_volatility": iv })),
        Err(e) => HttpResponse::BadRequest().body(format!("IV Processing Error: {}", e)),
    }
}

#[get("/historical-volatility")]
async fn check_historical_vol() -> impl Responder {
    let file_path = "historical_prices.txt";
    match volatility::parse_price_file(file_path) {
        Ok(prices) => {
            if let Some(vol) = volatility::calculate_historical_volatility(&prices) {
                HttpResponse::Ok().json(serde_json::json!({
                    "status": "success",
                    "parsed_records": prices.len(),
                    "calculated_annualized_volatility": vol
                }))
            } else {
                HttpResponse::BadRequest().body("Row bounds verification error.")
            }
        }
        Err(_) => HttpResponse::NotFound().body("Missing pricing file."),
    }
}

// SERVE INTEGRATED FRONTEND DASHBOARD HTML
#[get("/")]
async fn serve_dashboard() -> impl Responder {
    let html = include_str!("index.html"); // Pulls the index file in at compilation
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!(
        "{}",
        "=== INITIALIZING M4 QUANT SUITE SERVER ==="
            .bright_green()
            .bold()
    );

    let mock_file = "historical_prices.txt";
    volatility::generate_mock_price_file(mock_file)?;

    let host = "0.0.0.0"; // Binds to all available interfaces to fix 127.0.0.1 loopbacks!
    let port = 8080;
    println!("Server live and listening on http://localhost:{}", port);

    HttpServer::new(|| {
        App::new()
            .service(serve_dashboard)
            .service(calculate_metrics)
            .service(calculate_iv_endpoint)
            .service(check_historical_vol)
    })
    .bind((host, port))?
    .run()
    .await
}

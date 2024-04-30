use ntex::web;

use ntex::service::{Middleware, Service, ServiceCtx};
use ntex::web::{types::State, Error, WebRequest, WebResponse};
use std::sync::Arc;

// Define a mock metrics recording function and state for demonstration purposes
mod metrics {
    use std::sync::Mutex;

    pub struct MetricsState {
        pub http_request_counter: Mutex<u64>,
    }

    pub fn record_http_request_metrics(status: &str, counter: &Mutex<u64>) {
        let mut num = counter.lock().unwrap();
        *num += 1;
        println!("Status: {}, Total Requests: {}", status, num);
    }
}

// Middleware for HTTP Metrics Logging
pub struct HttpMetrics;

impl<S> Middleware<S> for HttpMetrics {
    type Service = HttpMetricsMiddleware<S>;

    fn create(&self, service: S) -> Self::Service {
        HttpMetricsMiddleware { service }
    }
}

pub struct HttpMetricsMiddleware<S> {
    service: S,
}

impl<S, Err> Service<WebRequest<Err>> for HttpMetricsMiddleware<S>
where
    S: Service<WebRequest<Err>, Response = WebResponse, Error = Error>,
{
    type Response = WebResponse;
    type Error = Error;

    ntex::forward_poll_ready!(service);
    ntex::forward_poll_shutdown!(service);

    async fn call(
        &self,
        req: WebRequest<Err>,
        ctx: ServiceCtx<'_, Self>,
    ) -> Result<Self::Response, Self::Error> {
        // Attempt to access shared application state
        let metrics_state = req.app_state::<Arc<metrics::MetricsState>>().clone();

        // Error is here: Diagnostics:
        // 1. cannot move out of `req` because it is borrowed
        // move out of `req` occurs here [E0505]
        let res = ctx.call(&self.service, req).await?;
        // Record metrics if the state was successfully retrieved
        match metrics_state {
            Some(state) => {
                metrics::record_http_request_metrics(
                    &res.status().to_string(),
                    &state.http_request_counter,
                );
            }
            None => todo!("Handle the absence of shared state"),
        }

        println!("Hi from response {}", res.status());
        Ok(res)
    }
}

#[web::get("/")]
async fn hello() -> impl web::Responder {
    web::HttpResponse::Ok().body("Hello world!")
}

async fn manual_hello() -> impl web::Responder {
    web::HttpResponse::Ok().body("Hey there!")
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    web::HttpServer::new(|| {
        web::App::new()
            .wrap(HttpMetrics)
            .service(hello)
            .route("/hey", web::get().to(manual_hello))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

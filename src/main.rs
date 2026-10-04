use axum::{Json, Router, extract::State, routing::post};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Event {
    source: String,
    kind: String,
    status: String,
    message: String,
}

type AppState = Arc<Mutex<Vec<Event>>>;

fn app(state: AppState) -> Router {
    Router::new()
        .route("/events", post(create_event).get(list_events))
        .with_state(state)
}

#[tokio::main]
async fn main() {
    let state: AppState = Arc::new(Mutex::new(Vec::new()));
    let app = app(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    println!("listening on http://127.0.0.1:3000");

    axum::serve(listener, app).await.unwrap();
}

async fn create_event(State(state): State<AppState>, Json(event): Json<Event>) -> Json<Event> {
    state.lock().unwrap().push(event.clone());
    Json(event)
}

async fn list_events(State(state): State<AppState>) -> Json<Vec<Event>> {
    Json(state.lock().unwrap().clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn test_app() -> Router {
        let state: AppState = Arc::new(Mutex::new(Vec::new()));
        app(state)
    }

    #[tokio::test]
    async fn get_events_returns_empty_list_initially() {
        let app = test_app();

        let request = Request::builder()
            .method("GET")
            .uri("/events")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();

        let events: Vec<Event> = serde_json::from_slice(&body).unwrap();

        assert!(events.is_empty());
    }

    #[tokio::test]
    async fn post_event_returns_created_event() {
        let app = test_app();

        let request = Request::builder()
            .method("POST")
            .uri("/events")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{
                    "source": "nas",
                    "kind": "backup",
                    "status": "success",
                    "message": "backup completed"
                }"#,
            ))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();

        let event: Event = serde_json::from_slice(&body).unwrap();

        assert_eq!(event.source, "nas");
        assert_eq!(event.kind, "backup");
        assert_eq!(event.status, "success");
        assert_eq!(event.message, "backup completed");
    }

    #[tokio::test]
    async fn post_event_then_get_events() {
        let app = test_app();

        let post_request = Request::builder()
            .method("POST")
            .uri("/events")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{
                    "source": "nas",
                    "kind": "backup",
                    "status": "success",
                    "message": "nightly backup completed"
                }"#,
            ))
            .unwrap();

        let post_response = app.clone().oneshot(post_request).await.unwrap();

        assert_eq!(post_response.status(), StatusCode::OK);

        let get_request = Request::builder()
            .method("GET")
            .uri("/events")
            .body(Body::empty())
            .unwrap();

        let get_response = app.oneshot(get_request).await.unwrap();

        assert_eq!(get_response.status(), StatusCode::OK);

        let body = get_response.into_body().collect().await.unwrap().to_bytes();

        let events: Vec<Event> = serde_json::from_slice(&body).unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].source, "nas");
        assert_eq!(events[0].kind, "backup");
        assert_eq!(events[0].status, "success");
        assert_eq!(events[0].message, "nightly backup completed");
    }

    #[tokio::test]
    async fn multiple_posts_are_returned_by_get() {
        let app = test_app();

        for message in ["first event", "second event"] {
            let request = Request::builder()
                .method("POST")
                .uri("/events")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{
                        "source": "nas",
                        "kind": "backup",
                        "status": "success",
                        "message": "{message}"
                    }}"#
                )))
                .unwrap();

            let response = app.clone().oneshot(request).await.unwrap();

            assert_eq!(response.status(), StatusCode::OK);
        }

        let request = Request::builder()
            .method("GET")
            .uri("/events")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        let body = response.into_body().collect().await.unwrap().to_bytes();

        let events: Vec<Event> = serde_json::from_slice(&body).unwrap();

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].message, "first event");
        assert_eq!(events[1].message, "second event");
    }

    #[tokio::test]
    async fn invalid_json_returns_client_error() {
        let app = test_app();

        let request = Request::builder()
            .method("POST")
            .uri("/events")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{
                    "source": "nas",
                    "kind":
                }"#,
            ))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert!(response.status().is_client_error());
    }
}

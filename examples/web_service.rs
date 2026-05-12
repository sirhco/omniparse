//! Example web service using Axum and Omniparse
//!
//! This example demonstrates how to build a REST API that accepts file uploads
//! and uses omniparse to extract content and metadata.
//!
//! Run with:
//! ```bash
//! cargo run --example web_service
//! ```
//!
//! Test with:
//! ```bash
//! curl -X POST -F "file=@test_data/text/sample.json" http://localhost:3000/parse
//! curl -X POST -F "file=@test_data/text/sample.csv" http://localhost:3000/parse?format=json
//! curl -X POST -F "file=@test_data/document/sample.docx" http://localhost:3000/detect
//! ```

use axum::{
    extract::{Multipart, Query},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use omniparse::{extract_from_bytes, detection::TypeDetector};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::str::FromStr;

#[tokio::main]
async fn main() {
    // Build the router
    let app = Router::new()
        .route("/", get(root))
        .route("/parse", post(parse_file))
        .route("/detect", post(detect_file))
        .route("/health", get(health_check));

    // Bind address. Default is localhost-only so a `cargo run --example
    // web_service` doesn't unexpectedly expose the port. Override with
    // `OMNIPARSE_BIND=0.0.0.0:3000` in containers / production.
    let bind = std::env::var("OMNIPARSE_BIND")
        .unwrap_or_else(|_| "127.0.0.1:3000".to_string());
    let addr = SocketAddr::from_str(&bind)
        .unwrap_or_else(|e| panic!("invalid OMNIPARSE_BIND={bind:?}: {e}"));
    println!("Omniparse web service listening on http://{}", addr);
    println!("\nEndpoints:");
    println!("  POST /parse   - Parse file and extract content");
    println!("  POST /detect  - Detect file type only");
    println!("  GET  /health  - Health check");
    println!("\nExample:");
    println!("  curl -X POST -F \"file=@test_data/text/sample.json\" http://{}/parse", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// Root endpoint
async fn root() -> &'static str {
    "Omniparse Web Service\n\nEndpoints:\n  POST /parse - Parse file\n  POST /detect - Detect file type\n  GET /health - Health check"
}

// Health check endpoint
async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        service: "omniparse-web".to_string(),
    })
}

// Parse file endpoint
async fn parse_file(
    Query(params): Query<ParseParams>,
    mut multipart: Multipart,
) -> Result<Json<ParseResponse>, AppError> {
    // Extract file from multipart form
    let (filename, data) = extract_file_from_multipart(&mut multipart).await?;

    // Parse the file
    let result = extract_from_bytes(&data, None)
        .map_err(|e| AppError::ParseError(e.to_string()))?;

    // Build response
    let response = ParseResponse {
        filename,
        mime_type: result.mime_type.clone(),
        detection_confidence: result.detection_confidence,
        metadata: serde_json::to_value(&result.metadata).unwrap_or(serde_json::Value::Null),
        content: if params.metadata_only {
            None
        } else {
            Some(match result.content {
                omniparse::core::Content::Text(text) => ContentResponse::Text(text),
                omniparse::core::Content::Binary(data) => ContentResponse::Binary {
                    size: data.len(),
                    preview: format!("{:02x?}", &data[..data.len().min(32)]),
                },
                omniparse::core::Content::None => ContentResponse::None,
            })
        },
    };

    Ok(Json(response))
}

// Detect file type endpoint
async fn detect_file(mut multipart: Multipart) -> Result<Json<DetectionResponse>, AppError> {
    // Extract file from multipart form
    let (filename, data) = extract_file_from_multipart(&mut multipart).await?;

    // Detect file type
    let detector = TypeDetector::new();
    let result = detector.detect_from_bytes(&data);

    // Build response
    let response = DetectionResponse {
        filename,
        mime_type: result.mime_type,
        confidence: result.confidence,
        detected_by: format!("{:?}", result.detected_by),
    };

    Ok(Json(response))
}

// Helper function to extract file from multipart form
async fn extract_file_from_multipart(multipart: &mut Multipart) -> Result<(String, Vec<u8>), AppError> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::MultipartError(e.to_string()))?
    {
        if field.name() == Some("file") {
            let filename = field
                .file_name()
                .unwrap_or("unknown")
                .to_string();
            
            let data = field
                .bytes()
                .await
                .map_err(|e| AppError::MultipartError(e.to_string()))?
                .to_vec();

            return Ok((filename, data));
        }
    }

    Err(AppError::MissingFile)
}

// Request/Response types

#[derive(Deserialize)]
struct ParseParams {
    #[serde(default)]
    metadata_only: bool,
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    service: String,
}

#[derive(Serialize)]
struct ParseResponse {
    filename: String,
    mime_type: String,
    detection_confidence: f32,
    metadata: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<ContentResponse>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum ContentResponse {
    Text(String),
    Binary { size: usize, preview: String },
    None,
}

#[derive(Serialize)]
struct DetectionResponse {
    filename: String,
    mime_type: String,
    confidence: f32,
    detected_by: String,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    message: String,
}

// Error handling

enum AppError {
    MultipartError(String),
    MissingFile,
    ParseError(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_type, message) = match self {
            AppError::MultipartError(msg) => (
                StatusCode::BAD_REQUEST,
                "multipart_error",
                msg,
            ),
            AppError::MissingFile => (
                StatusCode::BAD_REQUEST,
                "missing_file",
                "No file provided in request".to_string(),
            ),
            AppError::ParseError(msg) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "parse_error",
                msg,
            ),
        };

        let body = Json(ErrorResponse {
            error: error_type.to_string(),
            message,
        });

        (status, body).into_response()
    }
}

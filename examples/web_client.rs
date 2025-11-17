//! Example client for the Omniparse web service
//!
//! This demonstrates how to interact with the web service programmatically.
//!
//! Run the server first:
//! ```bash
//! cargo run --example web_service
//! ```
//!
//! Then run this client:
//! ```bash
//! cargo run --example web_client
//! ```

use serde::Deserialize;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let client = reqwest::Client::new();
    let base_url = "http://localhost:3000";

    println!("🔌 Omniparse Web Service Client");
    println!("================================\n");

    // Test health check
    println!("1. Checking service health...");
    let health: HealthResponse = client
        .get(format!("{}/health", base_url))
        .send()
        .await?
        .json()
        .await?;
    println!("   Status: {}\n", health.status);

    // Parse a JSON file
    println!("2. Parsing JSON file...");
    let json_data = std::fs::read("test_data/text/sample.json")?;
    let form = reqwest::multipart::Form::new()
        .part("file", reqwest::multipart::Part::bytes(json_data)
            .file_name("sample.json"));
    
    let parse_response: ParseResponse = client
        .post(format!("{}/parse", base_url))
        .multipart(form)
        .send()
        .await?
        .json()
        .await?;
    
    println!("   Filename: {}", parse_response.filename);
    println!("   MIME Type: {}", parse_response.mime_type);
    println!("   Confidence: {:.2}", parse_response.detection_confidence);
    println!("   Metadata keys: {}", parse_response.metadata.as_object().map(|m| m.len()).unwrap_or(0));
    println!();

    // Detect file type
    println!("3. Detecting DOCX file type...");
    let docx_data = std::fs::read("test_data/document/sample.docx")?;
    let form = reqwest::multipart::Form::new()
        .part("file", reqwest::multipart::Part::bytes(docx_data)
            .file_name("sample.docx"));
    
    let detect_response: DetectionResponse = client
        .post(format!("{}/detect", base_url))
        .multipart(form)
        .send()
        .await?
        .json()
        .await?;
    
    println!("   Filename: {}", detect_response.filename);
    println!("   MIME Type: {}", detect_response.mime_type);
    println!("   Confidence: {:.2}", detect_response.confidence);
    println!("   Detected By: {}", detect_response.detected_by);
    println!();

    // Parse with metadata only
    println!("4. Parsing CSV with metadata only...");
    let csv_data = std::fs::read("test_data/text/sample.csv")?;
    let form = reqwest::multipart::Form::new()
        .part("file", reqwest::multipart::Part::bytes(csv_data)
            .file_name("sample.csv"));
    
    let metadata_response: ParseResponse = client
        .post(format!("{}/parse?metadata_only=true", base_url))
        .multipart(form)
        .send()
        .await?
        .json()
        .await?;
    
    println!("   Filename: {}", metadata_response.filename);
    println!("   MIME Type: {}", metadata_response.mime_type);
    println!("   Has content: {}", metadata_response.content.is_some());
    println!();

    println!("✅ All requests completed successfully!");

    Ok(())
}

// Response types (matching the server)

#[derive(Deserialize)]
struct HealthResponse {
    status: String,
}

#[derive(Deserialize)]
struct ParseResponse {
    filename: String,
    mime_type: String,
    detection_confidence: f32,
    metadata: serde_json::Value,
    content: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct DetectionResponse {
    filename: String,
    mime_type: String,
    confidence: f32,
    detected_by: String,
}

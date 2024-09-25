use reqwest::{multipart, Client};
use std::path::Path;
use tokio::{self, fs};

const FILEPATH: &'static str = "./temp/image.jpg";


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Define the file you want to upload
    let file_content = fs::read(FILEPATH).await?;

    // Create a new HTTP client
    let client = Client::new();

    let file_part = multipart::Part::bytes(file_content).mime_str("application/octet-stream")?;


    // Prepare the multipart form, including the file
    let form = reqwest::multipart::Form::new()
        .part("file", file_part); 

    // Send the file to the Actix server (adjust the URL to match your server)
    let response = client
        .post("http://127.0.0.1:8080/submit-image/12") // Upload URL from your Actix server
        .multipart(form) // Attach the multipart form
        .send()
        .await?; // Send the request and await the response

    // Check the response from the server
    if response.status().is_success() {
        println!("File uploaded successfully!");
    } else {
        println!("File upload failed. Status: {}", response.status());
    }

    Ok(())
}
use fal_rust::{
    client::{ClientCredentials, FalClient},
    utils::download_image,
};
use serde::{Deserialize, Serialize};

use dotenv::dotenv;


#[derive(Debug, Serialize, Deserialize)]
struct ImageResult {
    url: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Output {
    images: Vec<ImageResult>,
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    let fal_token = std::env::var("FALAI_KEY").expect("FALAI_KEY must be set.");


    let client = FalClient::new(ClientCredentials::Key(fal_token));

    let res = client
        .run(
            "fal-ai/fast-sdxl",
            serde_json::json!({
                "prompt": "A large waterfall in the middle of a volcano, surrounded by lush greenery and children's playground equipment.",
                "image_size": "landscape_4_3"

            }),
        )
        .await
        .unwrap();

    let output: Output = res.json::<Output>().await.unwrap();

    let url = output.images[0].url.clone();
    let filename = url.split('/').last().unwrap();

    download_image(&url, format!("{}/{}", "temp", filename).as_str())
        .await
        .unwrap();
}
use serde::{Deserialize, Serialize};


#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TaskDefinition {
    TextGeneration(TextGenerationSettings),
    ImageGeneration(ImageGenerationSettings)
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TextGenerationSettings {
    pub model: String
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImageGenerationSettings {
    
}



#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TaskPayload {
    TextGeneration(TextGenerationPayload),
    ImageGeneration(ImageGenerationPayload)
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TextGenerationPayload {
    pub system_prompt: String,
    pub user_prompt: String
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImageGenerationPayload {
    
}
use serde::{Deserialize, Serialize};


#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TaskDefinition {
    TextGeneration(TextGenerationSettings),
    ImageGeneration(ImageGenerationSettings),
    VoiceGeneration(VoiceGenerationSettings)
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TextGenerationSettings {
    pub model: String
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImageGenerationSettings {
    pub model: String

}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VoiceGenerationSettings {
    pub model: String

}




#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TaskPayload {
    TextGeneration(TextGenerationPayload),
    ImageGeneration(ImageGenerationPayload),
    VoiceGeneration(VoiceGenerationPayload)
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TextGenerationPayload {
    pub system_prompt: String,
    pub user_prompt: String
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AspectRatio {
    Portrait, //= "Portrait",
    Landscape, //= "Landscape",
    Square //= "Square"
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImageGenerationPayload {
    pub positive_prompt: String,
    pub negative_prompt: String,
    pub aspect_ratio: AspectRatio,
    pub config_scale: u32,
    pub nb_steps: u32
}



#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VoiceGenerationPayload {
    pub prompt: String,
    pub voice: String
}

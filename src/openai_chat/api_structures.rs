use serde::{Serialize, Deserialize};
use serde_json::Value;

#[derive(Serialize, Deserialize)]
pub struct RequestBody {
    pub model: String,
    pub messages: Vec<Message>,
    pub stream: bool,
    pub stream_options: StreamOptions,
}

#[derive(Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: Vec<ContentItem>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentItem {
    Text {
        text: String
    },
    ImageUrl {
        image_url: ImageUrl
    },
}

#[derive(Serialize, Deserialize)]
pub struct ImageUrl {
    pub url: String,
}

#[derive(Serialize, Deserialize)]
pub struct StreamOptions {
    #[serde(rename = "include_usage")]
    pub include_usage: bool,
}


#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseBody {
    pub choices: Vec<Choice>,
    pub object: String,
    pub usage: Option<TokenUsage>,
    pub created: i64,
    #[serde(rename = "system_fingerprint")]
    pub system_fingerprint: Option<String>,
    pub model: String,
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Choice {
    #[serde(default)]
    pub delta: Delta,
    #[serde(rename = "finish_reason")]
    pub finish_reason: Option<FinishReason>,
    pub index: i32,
    pub logprobs: Option<Value>, // 根据实际情况可定义具体类型
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Delta {
    #[serde(default)]
    pub content: Option<String>,
    #[serde(rename = "reasoning_content")]
    pub reasoning_content: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    Stop,
    Length,
    ContentFilter,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenUsage {
    #[serde(rename = "prompt_tokens")]
    pub prompt_tokens: u32,
    #[serde(rename = "completion_tokens")]
    pub completion_tokens: u32,
    #[serde(rename = "total_tokens")]
    pub total_tokens: u32,
    #[serde(rename = "completion_tokens_details")]
    pub completion_details: Option<TokenDetails>,
    #[serde(rename = "prompt_tokens_details")]
    pub prompt_details: Option<TokenDetails>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenDetails {
    #[serde(rename = "text_tokens")]
    pub text_tokens: u32,
    #[serde(rename = "image_tokens")]
    pub image_tokens: Option<u32>, // 仅出现在 prompt 的 details 中
}
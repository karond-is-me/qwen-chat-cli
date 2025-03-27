use reqwest::header;
use termimad::MadSkin;
use  termimad::terminal_size;
use serde::{Deserialize, Serialize};
use base64::{engine::general_purpose, Engine as _};
use arboard::Clipboard;
use image::{DynamicImage, ImageBuffer, ImageFormat, RgbaImage};
use infer;
use std::io::{self, Write};
use tokio::time::{sleep, Duration};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::env;
use dotenv::dotenv;

const SKIN_STR:&str = r###"
    {
    "bold": "#fb0 bold",
    "italic": "dim italic",
    "strikeout": "crossedout red",
    "bullet": "○ yellow bold",
    "paragraph": "gray(20) 4 4",
    "code_block": "gray(2) gray(15) 4",
    "headers": [
    "yellow bold center",
    "yellow underlined",
    "yellow"
    ],
    "quote": "> red",
    "horizontal-rule": "~ #00cafe",
    "table": "#540 center",
    "scrollbar": "red yellow"
    }
    "###;

// 定义消息类型
#[derive(Debug, Serialize, Deserialize,Clone)]
struct Message {
    role: String,
    content: Vec<Content>,
}

#[derive(Debug, Serialize, Deserialize,Clone)]
struct Content {
    #[serde(rename = "type")]
    kind: String,
    #[serde(flatten)] 
    content: ContentData,
}

#[derive(Debug, Serialize, Deserialize,Clone)]
#[serde(untagged)]
enum ContentData{
    Text { text: String },
    ImageUrl { image_url: ImageUrl },
}

#[derive(Debug, Serialize, Deserialize,Clone)]
struct ImageUrl {
    url: String,
}

// OpenAI API 请求结构
#[derive(Debug, Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<Message>
}

// OpenAI API 响应结构
#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: MessageResponse,
}

#[derive(Debug, Deserialize)]
struct MessageResponse {
    #[allow(dead_code)]
    role: String,
    content: String,
}

struct OpenAiChat {
    model: String,
    messages: Vec<Message>,
    base_url: String,
    api_key: String,
    client: reqwest::Client,
    skin: MadSkin
}

impl OpenAiChat {
    fn new(base_url:String, model: String, api_key: String) -> Self {
        Self {
            model,
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: vec![Content{
                        kind: "text".to_string(),
                        content: ContentData::Text {
                            text: "你是一个AI智能助手，请务必根据用户的指令回答".to_string(),
                        }
                    }],
                }
            ],
            base_url,
            api_key,
            client: reqwest::Client::new(),
            skin : serde_json::from_str(SKIN_STR).unwrap()
        }
    }
    fn add_text(&mut self, role: &str, text: &str) {
        self.messages.push(Message {
            role: role.to_string(),
            content: vec![Content{
                kind: "text".to_string(),
                content: ContentData::Text {
                    text: text.to_string(),
                }
            }]
        });
    }
    // 添加图像消息 (支持本地文件或URL)
    async fn add_image(&mut self, role: &str, image_source: &str) -> Result<(), String> {
        let content = if image_source.starts_with("http") {
            // 如果是URL
            vec![Content{ 
                kind: "image_url".to_string(),
                content: ContentData::ImageUrl {
                    image_url: ImageUrl {
                        url: image_source.to_string(),
                    },
                }
              }
            ]
        } else {
            // 如果是本地文件
            let image_data: Vec<u8> = std::fs::read(image_source).map_err(|e| e.to_string())?;
            let base64_image = general_purpose::STANDARD.encode(&image_data);
            let kind = infer::get(&image_data).expect("file type is known");
            let mime_type = kind.mime_type();
            
            vec![Content{ 
                kind: "image_url".to_string(),
                content: ContentData::ImageUrl {
                    image_url: ImageUrl {
                        url: format!("data:{};base64,{}", mime_type, base64_image)
                    },
                }
              }
            ]
        };

        self.messages.push(Message {
            role: role.to_string(),
            content,
        });
        Ok(())
    }
    // 添加图像消息 (支持本地文件或URL)
    async fn add_raw_image(&mut self, role: &str, image_data: Vec<u8>) -> Result<(), String> {
        
        let base64_image = general_purpose::STANDARD.encode(&image_data);
        let kind = infer::get(&image_data).expect("file type is known");
        let mime_type = kind.mime_type();
        
        let content = vec![Content{ 
            kind: "image_url".to_string(),
            content: ContentData::ImageUrl {
                image_url: ImageUrl {
                    url: format!("data:{};base64,{}", mime_type, base64_image)
                },
            }
            }
        ];

        self.messages.push(Message {
            role: role.to_string(),
            content,
        });
        Ok(())
    }
    // 发送请求到OpenAI
    async fn send(&mut self) -> Result<String, String> {
        let request_body = OpenAIRequest {
            model: self.model.clone(),
            messages: self.messages.clone()
        };
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();
    
        // 启动 spinner 线程
        tokio::spawn(async move {
            let spin_chars = vec!['-', '\\', '|', '/'];
            let mut i = 0;
            while running_clone.load(Ordering::Relaxed) {
                print!("\rLoading {} ", spin_chars[i % spin_chars.len()]);
                std::io::stdout().flush().unwrap();
                i += 1;
                sleep(Duration::from_millis(100)).await;
            }
        });
        let response = self.client
            .post(self.base_url.clone())
            .timeout(Duration::from_secs(3*60))
            .header(header::AUTHORIZATION, format!("Bearer {}", self.api_key))
            .header(header::CONTENT_TYPE, "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !response.status().is_success() {
            return Err(format!("API request failed: {}", response.status()));
        }
        running.store(false, Ordering::Relaxed); // 停止 spinner
        let response_body: OpenAIResponse = response.json().await.map_err(|e| e.to_string())?;

        if let Some(choice) = response_body.choices.first() {
            let assistant_message = &choice.message.content;
            self.add_text("assistant", assistant_message);
            Ok(assistant_message.clone())
        } else {
            Err("No response from API".to_string())
        }
    }

    async  fn conversation(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut input = String::new();
        loop {
            println!("请输入问题或指令(clear;image;clip;exit)：");
            input.clear(); // 清空之前的输入
    
            io::stdin()
                .read_line(&mut input)
                .expect("无法读取输入");
            input = input.trim().to_string();
            if input.starts_with("/clear") {
                self.clear();
                continue;
            }else if input.starts_with("/image") {
                let image_path = input.strip_prefix("/image")
                .ok_or("无法解析命令")?
                .trim();
        
                // 检查路径是否为空
                if image_path.is_empty() {
                    return Err("请输入图片路径".into());
                }
                self.add_image("user", image_path).await?;
                println!("图片加载成功！");
            }else if input.starts_with("/clip") || input == "C"{
                let mut clipboard = Clipboard::new().unwrap();
                let img_res = clipboard.get_image();
                if img_res.is_err(){
                    println!("剪贴板中没有图片！");
                    continue;
                }
                let image_data = img_res.unwrap();
                let width = image_data.width as u32;
                let height = image_data.height as u32;
                let rgba_image: RgbaImage = ImageBuffer::from_raw(width, height, image_data.bytes.to_vec()).unwrap();
                // 将 RGBA 图像转换为 RGB 图像
                let rgb_image = DynamicImage::ImageRgba8(rgba_image).to_rgb8();

                // 编码为 JPEG 格式
                let mut buffer = Vec::new();
                    // 编码为 JPEG 并写入缓冲区
                rgb_image.write_to(&mut std::io::Cursor::new(&mut buffer), ImageFormat::Jpeg)
                    .expect("JPEG encoding failed");
                self.add_raw_image("user", buffer).await?;
                println!("图片读取成功！");
            }else if input.starts_with("/exit"){
                break;
            }else{
                self.add_text("user", &input);
                let response = self.send().await?;
                io::stdout().flush().unwrap();
                print!("\r");
                io::stdout().flush().unwrap();
                self.skin.print_text(response.as_str());
                let (w,_)= terminal_size();
                println!("{}", "─".repeat(w.into()));
            }
        }
        Ok(())
    }

    // 清空对话历史
    fn clear(&mut self) {
        self.messages.clear();
        self.messages.push(Message {
            role: "system".to_string(),
            content: vec![Content{
                kind: "text".to_string(),
                content: ContentData::Text {
                    text: "你是一个AI智能助手，请务必根据用户的指令回答".to_string(),
                }
            }],
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let api_key = env::var("API_KEY").expect("API_KEY must be set");
    let mut model_name = "qwen2.5-vl-32b-instruct".to_string();
    let env_model = env::var("MODEL");
    if env_model.is_ok(){
        model_name = env_model.unwrap();
    }
    let mut base_url = "https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions".to_string();
    let env_url = env::var("BASE_URL");
    if env_url.is_ok(){
        base_url = env_url.unwrap();
    }
    let mut conversation = OpenAiChat::new(base_url,model_name,api_key);
    conversation.conversation().await?;
    Ok(())
}
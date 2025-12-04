use anthropoki::{
    AnthropicClient, Content, InputMessage, MessagesRequest, MessagesRequestBody, Role,
};

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    let client = AnthropicClient::default();

    let response = client
        .messages(&MessagesRequest {
            x_api_key: std::env::var("ANTHROPIC_API_KEY").unwrap().into(),
            body: MessagesRequestBody {
                messages: vec![InputMessage {
                    role: Role::User,
                    content: Content::String("Hello, Anthropic!".to_string()),
                    ..Default::default()
                }],
                ..Default::default()
            },
            ..Default::default()
        })
        .await?;

    eprintln!("Response: {:?}", response);

    Ok(())
}

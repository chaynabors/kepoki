use anthropoki::{
    AnthropicClient, Content, InputMessage, MessagesRequest, MessagesRequestBody, Role,
};

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    let client = AnthropicClient::default();

    let mut response = client
        .messages_stream(&MessagesRequest {
            x_api_key: std::env::var("ANTHROPIC_API_KEY").unwrap().into(),
            body: MessagesRequestBody {
                messages: vec![InputMessage {
                    role: Role::User,
                    content: Content::String("Hello, Anthropic!".to_string()),
                    ..Default::default()
                }],
                stream: true,
                ..Default::default()
            },
            ..Default::default()
        })
        .await?;

    while let Some(chunk) = response.recv().await? {
        eprintln!("Chunk: {:?}", chunk);
    }

    Ok(())
}

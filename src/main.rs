use chrono::Utc;
use comfyui_rs::*;
use log::{error, info, LevelFilter};
use ollama_rs::{generation::completion::request::GenerationRequest, Ollama};
use poise::{
    serenity_prelude::{self as serenity, CreateAttachment, CreateEmbed, CreateEmbedFooter},
    CreateReply,
};

struct Data {} // User data, which is stored and accessible in all command invocations
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[derive(Debug, poise::ChoiceParameter)]
pub enum Models {
    #[name = "mistral"]
    Mistral,
    #[name = "caveman"]
    Caveman,
    #[name = "racist"]
    Racist,
    #[name = "lobotomy"]
    Lobotomy,
    #[name = "greentext"]
    Greentext,
}

impl std::fmt::Display for Models {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Models::Mistral => write!(f, "dolphin-mistral"),
            Models::Caveman => write!(f, "caveman-mistral"),
            Models::Racist => write!(f, "racist-mistral"),
            Models::Lobotomy => write!(f, "tinyllama:1.1b-chat-v0.6-q2_K"),
            Models::Greentext => write!(f, "greentext-mistral"),
        }
    }
}

#[poise::command(slash_command, prefix_command, user_cooldown = 10)]
async fn llm(
    ctx: Context<'_>,
    #[description = "Model"] model: Models,
    #[description = "Prompt"] prompt: String,
) -> Result<(), Error> {
    ctx.defer().await?;
    info!("Generating response for model `{model}` and prompt `{prompt}`...");
    let ollama = Ollama::default();
    let res = ollama
        .generate(GenerationRequest::new(model.to_string(), prompt))
        .await;

    match res {
        Ok(response) => {
            let footer = CreateEmbedFooter::new("Made by @DuckyBlender | Generated with Ollama");
            // token count / duration
            let response_speed = f32::from(response.final_data.clone().unwrap().eval_count)
                / (response.final_data.clone().unwrap().eval_duration as f32 / 1_000_000_000.0);

            let embed = CreateEmbed::default()
                .title(format!("Generated by `{model}`"))
                .description(response.response)
                .color(0x00ff00)
                .field(
                    "Duration",
                    format!(
                        "`{:.2}s`",
                        // total_durations is in nanoseconds
                        response.final_data.clone().unwrap().total_duration as f32
                            / 1_000_000_000.0
                    ),
                    true,
                )
                .field("Speed", format!("`{response_speed:.2} tokens/s`"), true)
                .footer(footer)
                .timestamp(Utc::now());
            let message = CreateReply::default().embed(embed);
            ctx.send(message).await?;
            info!("Response sent successfully");
        }
        Err(e) => {
            let embed = CreateEmbed::default()
                .title("Error generating response")
                .description(format!("Error: {e}"))
                .color(0xff0000)
                .timestamp(Utc::now());
            let message = CreateReply::default().embed(embed);
            ctx.send(message).await?;
            error!("Failed to generate response: {:?}", e);
        }
    }

    Ok(())
}

#[poise::command(slash_command, prefix_command, user_cooldown = 10)]
async fn img(
    ctx: Context<'_>,
    #[description = "Steps"]
    // #[choices(1, 4)]
    // steps: u32,
    #[description = "Prompt"] prompt: String,
) -> Result<(), Error> {
    info!("Generating image for prompt `{prompt}`...");
    ctx.defer().await?;
    let client = comfyui_rs::Client::new("127.0.0.1:8188");
    let json_prompt = include_str!("../workflow_api.json");
    // Convert to JSON value
    let mut json_prompt: serde_json::Value = serde_json::from_str(json_prompt).unwrap();
    // Change the prompt
    json_prompt["6"]["inputs"]["text"] = serde_json::Value::String(prompt.clone());
    json_prompt["13"]["inputs"]["noise_seed"] =
        serde_json::Value::Number(serde_json::Number::from(rand::random::<u64>()));
    // json_prompt["22"]["inputs"]["steps"] =
    //     serde_json::Value::Number(serde_json::Number::from(steps));

    let now = std::time::Instant::now();
    let images = client.get_images(json_prompt).await.unwrap();
    let elapsed = now.elapsed().as_millis();
    info!("Image generated successfully in {elapsed}ms");
    // Send this as an attachment
    // let attachment = CreateAttachment::bytes(image, "crong.png");
    let attachments = images
        .iter()
        .map(|(filename, bytes)| CreateAttachment::bytes(bytes.clone(), filename))
        .collect::<Vec<_>>();

    // For now just send the first image (because we're generating one image)
    // I'm not sure if it's even possible to send multiple images in a single message
    let footer = CreateEmbedFooter::new("Made by @DuckyBlender | Generated with SDXL-Turbo");
    let message = CreateReply::default()
        .attachment(attachments[0].clone())
        .embed(
            CreateEmbed::default()
                .title("SDXL-Turbo")
                .fields(vec![
                    ("Prompt", format!("`{prompt}`"), true),
                    (
                        "Duration",
                        format!("`{:.2}s`", elapsed as f32 / 1000.0),
                        true,
                    ),
                    // ("Steps", format!("`{steps}`"), true),
                ])
                .color(0x00ff00)
                .footer(footer)
                .timestamp(Utc::now()),
        );
    ctx.send(message).await?;
    info!("Image sent successfully");

    Ok(())
}

#[poise::command(slash_command, prefix_command, user_cooldown = 1)]
async fn stats(ctx: Context<'_>) -> Result<(), Error> {
    info!("Getting stats...");
    let client = comfyui_rs::Client::new("127.0.0.1:8188");
    let stats: SystemStats = client.get_system_stats().await.unwrap();
    let embed = CreateEmbed::default()
        .title("Stats")
        .fields(vec![
            (
                "GPU",
                format!(
                    "{}\nVRAM: {}MB/{}MB",
                    stats.devices[0].name,
                    stats.devices[0].vram_free / 1_000_000, // convert to MB
                    stats.devices[0].vram_total / 1_000_000  // convert to MB
                ),
                true,
            ), // todo add more fields from /queue
        ])
        .color(0x0000_ff00)
        .timestamp(Utc::now());
    let message = CreateReply::default().embed(embed);
    ctx.send(message).await?;
    info!("Stats sent successfully");

    Ok(())
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    // Initialize env_logger with a custom configuration
    let mut builder = env_logger::Builder::new();
    builder.filter(None, LevelFilter::Warn); // Set the default level to Warn for all modules
    builder.filter(Some("duckgpt"), LevelFilter::Info);
    builder.filter(Some("comfyui_rs"), LevelFilter::Info);
    builder.init();

    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
    let intents = serenity::GatewayIntents::non_privileged();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![llm(), img(), stats()],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                let guild_id = serenity::GuildId::new(1175184892225671258);
                poise::builtins::register_in_guild(ctx, &framework.options().commands, guild_id)
                    .await?;
                Ok(Data {})
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;
    client.unwrap().start().await.unwrap();
}

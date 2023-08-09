use std::{env, vec};

use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::id::UserId;
use serenity::prelude::*;

use openai::{
    chat::{ChatCompletion, ChatCompletionMessage, ChatCompletionMessageRole},
    set_key,
};

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    // Set a handler for the `message` event - so that whenever a new message
    // is received - the closure (or function) passed will be called.
    //
    // Event handlers are dispatched through a threadpool, and so multiple
    // events can be dispatched simultaneously.
    async fn message(&self, ctx: Context, msgg: Message) {
        set_key(env::var("OPENAI_API_KEY").unwrap());
        let msg = msgg.content.replace("\\", "");
        let mut text_val: String = "".to_string();

        let v: Vec<&str> = vec![
            "!ping", "/hey", "/explain", "/simple", "/steps", "/recipe", "/help",
        ];

        let v2 = v.clone();

        for item in v {
            if msg.to_string().starts_with(item) {
                println!("{}: '{}'", item, msg.to_string());

                match msg.to_string().split_whitespace().next() {
                    Some("!ping") => {
                        // Sending a message can fail, due to a network error, an
                        // authentication error, or lack of permissions to post in the
                        // channel, so log to stdout when some error happens, with a
                        // description of it.
                        if let Err(why) = msgg.channel_id.say(&ctx.http, "Pong!").await {
                            println!("Error sending message: {:?}", why);
                        }
                    }
                    Some("/hey") => {
                        text_val = "You are a muppet expert.  All you want to talk about is muppets.  Your favorite muppet is kermit the frog, but you like mrs. piggy too.".to_string();
                    }
                    Some("/explain") => {
                        text_val = "explain.".to_string();
                    }
                    Some("/steps") => {
                        text_val = "break this out into steps.".to_string();
                    }
                    Some("/simple") => {
                        text_val = "explain in a simple and consise way. give analogies a beginner might understand.".to_string();
                    }
                    Some("/recipe") => {
                        text_val = "Respond with a recipie if this prompt has food. If it does not have food, return 'gimmie some food to work with'.".to_string();
                    }
                    Some("/help") => {
                        let mut help_text = "Available commands:\n".to_string();
                        for command in &v2 {
                            help_text.push_str(&format!("- {}\n", command));
                        }
                        if let Err(why) = msgg.channel_id.say(&ctx.http, help_text).await {
                            println!("Error sending message: {:?}", why);
                        }
                    }
                    _ => {}
                }

                let mut messages = vec![ChatCompletionMessage {
                    role: ChatCompletionMessageRole::System,
                    content: Some(text_val.to_string()),
                    name: None,
                    function_call: None,
                }];

                let words: Vec<&str> = msg.split_whitespace().collect();
                // The user included additional words after "!ping"
                let extra_words = &words[1..];

                messages.push(ChatCompletionMessage {
                    role: ChatCompletionMessageRole::User,
                    content: Some(extra_words.join(" ")),
                    name: None,
                    function_call: None,
                });

                let chat_completion = ChatCompletion::builder("gpt-3.5-turbo", messages.clone())
                    .create()
                    .await
                    .unwrap();
                let returned_message = chat_completion.choices.first().unwrap().message.clone();

                if let Err(why) = msgg
                    .channel_id
                    .say(&ctx.http, &returned_message.content.clone().unwrap().trim())
                    .await
                {
                    println!("Error sending message: {:?}", why);
                }
            }
        }
    }

    // Set a handler to be called on the `ready` event. This is called when a
    // shard is booted, and a READY payload is sent by Discord. This payload
    // contains data like the current user's guild Ids, current user data,
    // private channels, and more.
    //
    // In this case, just print what the current user's username is.
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[tokio::main]
async fn main() {
    // Configure the client with your Discord bot token in the environment.
    let token = env::var("DISCORD_MUPPET_FRIEND").expect("Expected a token in the environment");
    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    // Create a new instance of the Client, logging in as a bot. This will
    // automatically prepend your bot token with "Bot ", which is a requirement
    // by Discord for bot users.
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    // Finally, start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform
    // exponential backoff until it reconnects.
    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}


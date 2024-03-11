mod message_sender;
mod message_formatter;
mod database;

use futures::future::join_all;
use std::env;
use std::error::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use teloxide::{prelude::*, utils::command::BotCommands};
use tokio::signal;
use tokio::task::JoinHandle;

use crate::database::pull_database;
use crate::message_sender::BotMessenger;

const ENV_CHATID: &str = "CHAT_ID";
const ENV_BOTKEY: &str = "BOT_KEY";
const ENV_INTV: &str = "QUERY_MIN_TIME";
const ENV_DB_PREFIX: &str = "DB_";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let shutdown_signal = Arc::new(AtomicBool::new(true));
    shutdown_signal.store(false, Ordering::Release);

    let shutdown_signal_clone = shutdown_signal.clone();
    tokio::spawn(async move {
        signal::ctrl_c().await.unwrap();
        shutdown_signal_clone.store(true, Ordering::Release);
        println!("Shutdown signal received.");
    });

    let mut join_handlers: Vec<JoinHandle<()>> = vec![];

    let databases: Vec<String> = env::vars()
        .filter(|(key, _)| key.starts_with(ENV_DB_PREFIX))
        .map(|(_, val)| val)
        .collect();

    println!("Trying to connect to {} databases", databases.len());

    for db in databases {
        let shutdown_signal_clone: Arc<AtomicBool> = shutdown_signal.clone();
        let bot = Bot::new(env::var(ENV_BOTKEY).unwrap());
        let bot_messenger = BotMessenger::new(
            bot,
            ChatId(env::var(ENV_CHATID).unwrap().parse::<i64>().unwrap())
        );

        let handler = tokio::spawn(async move {
            let _ = pull_database(db,
                 shutdown_signal_clone,
                env::var(ENV_INTV).unwrap_or_else(|_| "5 sec".into()),
                bot_messenger).await;
        });
        join_handlers.push(handler);
    }

    let bot_handle = tokio::spawn(async move {
        let bot = Bot::new(env::var(ENV_BOTKEY).unwrap());
        Command::repl(bot, answer).await;
        println!("Bot is done");
    });
    join_handlers.push(bot_handle);

    join_all(join_handlers).await;
    println!("Shutdown complete.");
    Ok(())
}

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
enum Command {
    #[command(description = "display this text.")]
    Start,
    #[command(description = "display this text.")]
    Help,
    #[command(description = "get the chat id.")]
    ChatId,
}

async fn answer(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    match cmd {
        Command::Help | Command::Start => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?
        }
        Command::ChatId => {
            let chatid = msg.chat.id.0;
            bot.send_message(msg.chat.id, format!("{chatid}")).await?
        }
    };
    Ok(())
}

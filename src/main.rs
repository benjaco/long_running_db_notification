use chrono::{DateTime, Utc};
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use std::collections::HashSet;
use std::env;
use std::error::Error;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use teloxide::{prelude::*, types::ParseMode, utils::command::BotCommands};
use tokio::signal;
use tokio::{
    task::{self, JoinHandle},
    time,
};
use async_trait::async_trait;

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
        let bot_messenger = BotMessenger::new(bot);

        let handler = tokio::spawn(async move {
            let _ = pull_database(db, shutdown_signal_clone, bot_messenger).await;
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


#[async_trait]
trait Messenger {
    async fn send(&self, query: RunningQuery) -> Result<(), String>;
}

struct BotMessenger {
    bot: Bot,
}

impl BotMessenger {
    fn new(bot: Bot) -> Self {
        Self { bot }
    }
}

#[async_trait]
impl Messenger for BotMessenger {
    async fn send(&self, query: RunningQuery) -> Result<(), String> {
        let formated_msg = fmt::format(format_args!(
            "Query done \\- {} by {}: \n ```sql\n{}```",
            escape_markdown_v2(&time_diff_text(query.query_start)),
            escape_markdown_v2(&query.application_name),
            escape_markdown_v2(&query.query)
        ));
    
        let result = match &self.bot
            .send_message(
                ChatId(env::var(ENV_CHATID).unwrap().parse::<i64>().unwrap()),
                formated_msg,
            )
            .parse_mode(ParseMode::MarkdownV2)
            .send()
            .await
        {
            Ok(_) => Ok(()),
            Err(_error) => {
                println!("{_error:?}");
                Err("Couldnt send message".to_owned())
            }
        };
        result
    }
}








#[derive(sqlx::FromRow, PartialEq, Eq, Hash, Clone, Serialize, Deserialize, Debug)]
struct RunningQuery {
    query_start: chrono::DateTime<chrono::Utc>,
    pid: i32,
    application_name: String,
    query: String,
}

async fn pull_database<M>(
    database_conncection_string: String,
    shutdown_signal: Arc<AtomicBool>,
    messenger: M,
) -> Result<(), sqlx::Error>
where
    M: Messenger + Send + Sync + 'static,
{
    let database_conncection_string = database_conncection_string.to_owned();

    let forever = task::spawn(async move {

        let mut interval = time::interval(Duration::from_secs(1));

        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(&database_conncection_string)
            .await
            .unwrap();

        println!("Connected to database");

        let mut running_queries: HashSet<RunningQuery> = HashSet::from([]);

        loop {
            for _ in 0..3 {
                interval.tick().await;
                if shutdown_signal.load(Ordering::SeqCst) == true {
                    println!("Shutdown signal received, stopping task.");
                    pool.close().await;
                    return;
                }
            }

            // Default interval set to '5 seconds'
            let interval = env::var(ENV_INTV).unwrap_or_else(|_| "5 sec".into());

            let query = format!(
                "
                select query_start, pid, application_name, query
                FROM pg_stat_activity
                where state = 'active'
                and pid <> pg_backend_pid()
                and query_start < NOW() - INTERVAL '{}'
                ",
                interval
            );

            let result = sqlx::query_as::<_, RunningQuery>(&query)
                .fetch_all(&pool)
                .await;

            match result {
                Ok(rows) => {
                    let currently_queries: HashSet<RunningQuery> =
                        HashSet::from_iter(rows.iter().cloned());

                    let slowly_finnished_queries: HashSet<_> =
                        running_queries.difference(&currently_queries).collect();

                    for ele in slowly_finnished_queries {
                        let _ = messenger.send(ele.to_owned()).await;
                    }

                    running_queries = currently_queries;
                }
                _ => println!("Query error"),
            }
        }
    });

    let _ = forever.await;
    Ok(())
}

fn time_diff_text(from: DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = now - from;

    let days = duration.num_days();
    let hours = duration.num_hours() % 24;
    let minutes = duration.num_minutes() % 60;
    let seconds = duration.num_seconds() % 60;

    match (days, hours, minutes) {
        (d, _, _) if d > 0 => format!(
            "{} days, {} hours, {} minutes, and {} seconds",
            days, hours, minutes, seconds
        ),
        (_, h, _) if h > 0 => format!(
            "{} hours, {} minutes, and {} seconds",
            hours, minutes, seconds
        ),
        (_, _, m) if m > 0 => format!("{} minutes and {} seconds", minutes, seconds),
        _ => format!("{} seconds", seconds),
    }
}

fn escape_markdown_v2(text: &str) -> String {
    let mut escaped_text = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '_' | '*' | '[' | ']' | '(' | ')' | '~' | '`' | '>' | '<' | '#' | '+' | '-' | '='
            | '|' | '{' | '}' | '.' | '!' => {
                escaped_text.push('\\');
                escaped_text.push(c);
            }
            _ => escaped_text.push(c),
        }
    }
    escaped_text
}



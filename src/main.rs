use chrono::{DateTime, Utc};
use futures::future::join_all;
use futures::Future;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use std::collections::HashSet;
use std::error::Error;
use std::fmt;
use std::pin::Pin;
use std::time::Duration;
use teloxide::prelude::*;
use teloxide::types::ParseMode;
use tokio::{task, time};
use std::env;

const ENV_CHATID : &str = "CHAT_ID";
const ENV_BOTKEY : &str = "BOT_KEY";
const ENV_INTV : &str = "QUERY_MIN_TIME";
const ENV_DB_PREFIX : &str = "DB_";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {

    let mut db_pullers: Vec<Pin<Box<dyn Future<Output = Result<(), sqlx::Error>>>>> = vec![];

    let databases: Vec<String> = env::vars()
        .filter(|(key, _)| key.starts_with(ENV_DB_PREFIX))
        .map(|(_, val)| val)
        .collect();

    println!("Trying to connect to {} databases", databases.len());

    for db in databases {
        let future = pull_database(db);
        db_pullers.push(Box::pin(future));
    }

    join_all(db_pullers).await;

    Ok(())
}

#[derive(sqlx::FromRow, PartialEq, Eq, Hash, Clone, Serialize, Deserialize, Debug)]
struct RunningQuery {
    query_start: chrono::DateTime<chrono::Utc>,
    pid: i32,
    application_name: String,
    query: String,
}

async fn pull_database(database_conncection_string: String) -> Result<(), sqlx::Error> {
    let database_conncection_string = database_conncection_string.to_owned();

    let forever = task::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(3));

        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_conncection_string)
            .await
            .unwrap();

        println!("Connected to database");

        let mut running_queries: HashSet<RunningQuery> = HashSet::from([]);

        loop {
            interval.tick().await;

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
                        let cbv = send_msg(ele.to_owned()).await;
                        match cbv {
                            Ok(_) => {}
                            Err(error) => println!("{error}"),
                        }
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
    // Calculate difference
    let duration = now.signed_duration_since(from);

    // Formatting the output
    let days = duration.num_days();
    let hours = duration.num_hours() % 24;
    let minutes = duration.num_minutes() % 60;
    let seconds = duration.num_seconds() % 60;

    if days > 0 {
        return fmt::format(format_args!(
            "{} days, {} hours, {} minutes, and {} seconds",
            days, hours, minutes, seconds
        ));
    }
    if hours > 0 {
        return fmt::format(format_args!(
            "{} hours, {} minutes, and {} seconds",
            hours, minutes, seconds
        ));
    }
    if minutes > 0 {
        return fmt::format(format_args!("{} minutes and {} seconds", minutes, seconds));
    }

    return fmt::format(format_args!("{} seconds", seconds));
}

fn escape_markdown_v2(text: String) -> String {
    text.replace("_", "\\_")
        .replace("*", "\\*")
        .replace("[", "\\[")
        .replace("]", "\\]")
        .replace("(", "\\(")
        .replace(")", "\\)")
        .replace("~", "\\~")
        .replace("`", "\\`")
        .replace(">", "\\>")
        .replace("<", "\\<")
        .replace("#", "\\#")
        .replace("+", "\\+")
        .replace("-", "\\-")
        .replace("=", "\\=")
        .replace("|", "\\|")
        .replace("{", "\\{")
        .replace("}", "\\}")
        .replace(".", "\\.")
        .replace("!", "\\!")
}

async fn send_msg(query: RunningQuery) -> Result<(), String> {
    let formated_msg = fmt::format(format_args!(
        "Query done \\- {} by {}: \n ```sql\n{}```",
        escape_markdown_v2(time_diff_text(query.query_start)),
        escape_markdown_v2(query.application_name.to_owned()),
        escape_markdown_v2(query.query.to_owned())
    ));
    let bot = Bot::new(env::var(ENV_BOTKEY).unwrap());

    let result = match bot
        .send_message(ChatId(env::var(ENV_CHATID).unwrap().parse::<i64>().unwrap()), formated_msg)
        .parse_mode(ParseMode::MarkdownV2)
        .send()
        .await
    {
        Ok(_) => Ok(()),
        Err(_error) => {
            println!("{_error:?}");
            Err("Couldnt send message".to_owned())
        } ,
    };
    result
}

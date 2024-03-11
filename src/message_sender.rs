use crate::{database::RunningQuery, message_formatter::{escape_markdown_v2, time_diff_text}};

use async_trait::async_trait;
use std::fmt;
use teloxide::{prelude::*, types::ParseMode};

#[async_trait]
pub trait Messenger {
    async fn send(&self, query: RunningQuery) -> Result<(), String>;
}

pub struct BotMessenger {
    bot: Bot,
    chat_id: ChatId
}

impl BotMessenger {
    pub fn new(bot: Bot, chat_id: ChatId) -> Self {
        Self { bot, chat_id }
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

        let result = match &self
            .bot
            .send_message(
                self.chat_id,
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

use teloxide::{prelude::*, utils::command::BotCommands};

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
pub enum Command {
    #[command(description = "display this text.")]
    Start,
    #[command(description = "display this text.")]
    Help,
    #[command(description = "get the chat id.")]
    ChatId,
}

pub async fn answer(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
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

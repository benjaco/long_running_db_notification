# README for Rust Telegram Notifier for SQL Queries

## Introduction

This Rust application monitors long-running SQL queries and sends a notification to a specified Telegram chat when these queries are completed. It is designed to work with PostgreSQL databases and utilizes `tokio` for asynchronous operations, `sqlx` for database interactions, `teloxide` for sending Telegram messages, and `chrono` for time-related functions.

## Features

- **Database Monitoring**: Monitors multiple PostgreSQL databases for long-running queries.
- **Asynchronous Execution**: Fully asynchronous execution leveraging Rust's `tokio` runtime.
- **Telegram Notifications**: Sends customizable notifications to a Telegram chat using the bot API.
- **Environment Variable Configuration**: Uses environment variables for easy configuration of database connections, Telegram bot API key, and chat ID.

## Running with Docker

You can run this application using a pre-built Docker image, `benjaco/long_running_db_notification`, available on Docker Hub. 

### Prerequisites

- Docker installed on your system.
- A Telegram bot token and chat ID.
- Connection string(s) for your PostgreSQL database(s).

### Pulling the Image

```shell
docker pull benjaco/long_running_db_notification:latest
```

### Running the Container

You'll need to pass the environment variables for the Telegram bot token (`BOT_KEY`), the chat ID (`CHAT_ID`), and the database connection string(s) (`DB_<NAME>`). Replace `<NAME>` with a unique identifier for each database.

```shell
docker run -d \
  -e QUERY_MIN_TIME='5 sec' \
  -e CHAT_ID='your_chat_id' \
  -e BOT_KEY='your_bot_key' \
  -e DB_PRIMARY='your_primary_db_connection_string' \
  -e DB_SECONDARY='your_secondary_db_connection_string' \
  --name db_notification \
  benjaco/long_running_db_notification:latest
```
## Running it locally

### Prerequisites

Before you start, ensure you have Rust and Cargo installed on your machine. Additionally, you will need:

- A Telegram bot token (obtained by creating a bot with [@BotFather](https://t.me/botfather)).
- The chat ID of the Telegram chat where notifications should be sent.
- Connection strings for the PostgreSQL databases you want to monitor.

### Environment Variables

To run the application, you need to set the following environment variables:

- `CHAT_ID`: The Telegram chat ID where notifications will be sent.
- `BOT_KEY`: The Telegram bot token.
- `DB_<NAME>`: The connection string for each PostgreSQL database you want to monitor. Replace `<NAME>` with a unique identifier for each database.

Example:
```shell
export CHAT_ID='123456789'
export BOT_KEY='your_bot_token'
export DB_PRIMARY='postgres://user:password@localhost/dbname'
export DB_SECONDARY='postgres://user:password@localhost/otherdb'
```

### Installation

Clone the repository and navigate to the project directory:


```shell
git clone github.com/benjaco/long_running_db_notification.git
cd long_running_db_notification
```

Build the project using Cargo:

```shell
cargo build --release
```

### Usage

Run the application:

```shell
cargo run --release
```

The application will start monitoring the specified databases for long-running queries. When a query exceeds the defined duration threshold, a notification will be sent to the specified Telegram chat.

### Customization

- **Query Duration Threshold**: Modify the SQL query within `pull_database` function to change the threshold for long-running queries.
- **Notification Frequency**: Adjust the interval duration in the `pull_database` function to increase or decrease how often the database is polled.
- **Notification Message**: Customize the message format in `send_msg` to include more details or change the appearance of the Telegram message.


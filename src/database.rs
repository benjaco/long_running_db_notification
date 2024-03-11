use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::{
    task,
    time,
};

use crate::message_sender::Messenger;

#[derive(sqlx::FromRow, PartialEq, Eq, Hash, Clone, Serialize, Deserialize, Debug)]
pub struct RunningQuery {
    pub query_start: chrono::DateTime<chrono::Utc>,
    pub pid: i32,
    pub application_name: String,
    pub query: String,
}

pub async fn pull_database<M>(
    database_conncection_string: String,
    shutdown_signal: Arc<AtomicBool>,
    db_go_back: String,
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


            let query = format!(
                "
                select query_start, pid, application_name, query
                FROM pg_stat_activity
                where state = 'active'
                and pid <> pg_backend_pid()
                and query_start < NOW() - INTERVAL '{}'
                ",
                db_go_back
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

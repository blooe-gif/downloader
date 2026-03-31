use clap::{Parser, Subcommand};
use idm_rs::{ai, config, db, engine};
use sqlx::SqlitePool;
use std::{collections::HashMap, path::PathBuf};

#[derive(Parser, Debug)]
#[command(author, version, about = "IDM-style high performance downloader")]
struct Cli {
    #[arg(long, default_value = "idm.toml")]
    config: PathBuf,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Add {
        url: String,
        #[arg(long)]
        output: Option<String>,
    },
    Run,
    RunTask {
        id: i64,
    },
    List,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let cfg = config::Config::load_or_create(&cli.config)?;
    let pool = db::init_db(&cfg.db_path).await?;
    let downloader = engine::Downloader::new(cfg.clone(), pool.clone());

    match cli.command {
        Commands::Add { url, output } => {
            let priority = score_priority(&url, output.as_deref().unwrap_or(""));
            let id = downloader.enqueue(&url, output, priority).await?;
            println!("queued task {id}");
        }
        Commands::Run => {
            downloader.run_next().await?;
        }
        Commands::RunTask { id } => {
            let task = db::fetch_task(&pool, id).await?;
            downloader.run_task(task).await?;
        }
        Commands::List => list_tasks(&pool).await?,
    }

    Ok(())
}

fn score_priority(url: &str, output: &str) -> f64 {
    let mut rules = HashMap::new();
    rules.insert("critical".to_string(), 8.0);
    rules.insert("backup".to_string(), 4.0);
    rules.insert("patch".to_string(), 5.5);
    ai::priority_score(url, output, &rules)
}

async fn list_tasks(pool: &SqlitePool) -> anyhow::Result<()> {
    for t in db::list_tasks(pool).await? {
        println!(
            "#{: <4} {: <10} priority={:.2} size={}MB path={} created={}",
            t.id,
            t.status,
            t.priority,
            t.file_size / (1024 * 1024),
            t.output_path,
            t.created_at
        );
    }
    Ok(())
}

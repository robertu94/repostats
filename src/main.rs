#![allow(dead_code)]

use anyhow::{Error,Context};
use std::str::FromStr;
use sqlx::{ConnectOptions};
use sqlx::sqlite::{SqliteConnectOptions, SqliteConnection};

#[derive(serde::Deserialize)]
#[derive(Debug)]
struct TrafficClonesResponse {
    timestamp: chrono::DateTime<chrono::Utc>,
    count: i64,
    uniques: i64,
}
#[derive(serde::Deserialize)]
#[derive(Debug)]
struct TrafficResponse {
    count: i64,
    uniques: i64,
    clones: Vec<TrafficClonesResponse>
}

#[derive(serde::Deserialize)]
#[derive(Debug)]
struct RepoStatsConfig {
    token: String,
    db_path: std::path::PathBuf,
    repos: Vec<(String, String)>
}

async fn insert_or_ignore_repo(conn: &mut SqliteConnection, owner: &str, repo: &str) -> Result<i64, Error> {
   sqlx::query(r#"INSERT OR IGNORE INTO repos(repo, owner) VALUES (?,?)"#)
       .bind(repo)
       .bind(owner)
       .execute(&mut *conn).await?;
   sqlx::query_as::<_, (i64,)>(r#"SELECT id FROM repos where repo=? AND owner=?"#)
       .bind(repo)
       .bind(owner)
       .fetch_one(&mut *conn).await
       .map(|v| v.0)
       .map_err(|e| e.into())
}

async fn insert_or_ignore_counts(conn: &mut SqliteConnection, repo_id: i64, data: &TrafficClonesResponse) -> Result<(), Error> {
   sqlx::query(r#"INSERT INTO downloads(repo_id, date, total_downloads, unique_downloads)
                               VALUES (?,?,?,?)
                               ON CONFLICT(repo_id, date) DO UPDATE SET 
                                total_downloads=max(total_downloads, excluded.total_downloads),
                                unique_downloads=max(unique_downloads, excluded.unique_downloads)
                               "#)
       .bind(repo_id)
       .bind(data.timestamp.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
       .bind(data.count)
       .bind(data.uniques)
       .execute(&mut *conn).await?;
   Ok(())
}

async fn setup_database(conn: &mut SqliteConnection) -> Result<(), Error> {
    let metadata_def = r#"
        CREATE TABLE IF NOT EXISTS metadata (
            schema_verison INTEGER PRIMARY KEY
        );
    "#;
    sqlx::query(&metadata_def).execute(&mut *conn).await?;
    let version = sqlx::query_as::<_ ,(i64,)>("SELECT schema_verison FROM metadata")
        .fetch_one(&mut *conn)
        .await.map(|v| v.0 ).unwrap_or(0);

    if version < 1  {
        println!("initalizing database");
        let table_def = r#"
            CREATE TABLE IF NOT EXISTS repos (
                id INTEGER PRIMARY KEY,
                repo TEXT NOT NULL,
                owner TEXT NOT NULL,
                UNIQUE(repo, owner)
            );
            CREATE TABLE IF NOT EXISTS downloads (
                download_id INTEGER PRIMARY KEY,
                repo_id INTEGER,
                date TIMESTAMP NOT NULL,
                total_downloads INTEGER NOT NULL,
                unique_downloads INTEGER NOT NULL,
                FOREIGN KEY(repo_id) REFERENCES repos(id),
                UNIQUE(repo_id, date)
            );
            INSERT INTO metadata(schema_verison) VALUES (1);
        "#;
        sqlx::query(&table_def).execute(&mut *conn).await?;
    }
    Ok(())
}

async fn record_clones(client: &reqwest::Client, headers: &reqwest::header::HeaderMap, conn: &mut SqliteConnection, owner: &str, repo: &str) -> Result<i64, Error> {
    let url = format!("https://api.github.com/repos/{}/{}/traffic/clones", &owner, &repo);
    let traffic = client.get(&url)
        .headers(headers.clone())
        .send()
        .await
        .with_context(|| format!("failed to get url {}", &url))?
        .error_for_status()
        .with_context(|| format!("github returned error status for url {}: probably permissions related", &url))?
        .json::<TrafficResponse>()
        .await
        .with_context(|| format!("failed to parse response for url {}", &url))
        ?;
    let mut new_downloads = 0;
    for clones in traffic.clones {
        let repo_id = insert_or_ignore_repo(&mut *conn, &owner, &repo)
            .await
            .with_context(|| format!("failed to write clones for url {}", &url))?;
        insert_or_ignore_counts(&mut *conn, repo_id, &clones).await?;
        new_downloads += clones.count;
    }
    Ok(new_downloads)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let config_file = std::env::args().nth(1).unwrap_or("config.json".to_string());
    let config = std::fs::read_to_string(&config_file)?;
    let config : RepoStatsConfig = serde_json::from_str(&config)
        .with_context(|| format!("failed to parse config file {}", &config_file))?;

    let token = config.token;
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(reqwest::header::ACCEPT, "application/vnd.github+json".parse()?);
    headers.insert(reqwest::header::AUTHORIZATION, format!("Bearer {}", token).parse()?);
    headers.insert(reqwest::header::USER_AGENT, "reqwest".parse()?);


    let mut conn = SqliteConnectOptions::from_str(config.db_path.to_str().unwrap())?
        .create_if_missing(true)
        .foreign_keys(true)
        .connect().await?;
    setup_database(&mut conn).await?;

    let client = reqwest::Client::new();
    let mut total_downloads = 0;
    for (owner, repo) in config.repos {
        match record_clones(&client, &headers, &mut conn, &owner, &repo).await {
            Ok(new_downloads) => { total_downloads += new_downloads }
            Err(e) => println!("failed to record clones {}", e)
        }
    }
    println!("{} new downloads", total_downloads);


    Ok(())
}

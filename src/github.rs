use std::{cmp::Ordering, collections::HashMap};

use log::warn;
use poem_openapi::Object;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::version::VERSION;

pub struct GitHubTrafficsSync {
    pub client: GitHubClient,
    pub repos_to_sync: Vec<String>,
    pub db_file: String,
}

impl GitHubTrafficsSync {
    pub fn new(client: GitHubClient, db_file: &str) -> Self {
        Self {
            client,
            repos_to_sync: vec![],
            db_file: db_file.to_owned(),
        }
    }

    pub fn add_repo(&mut self, repo: &str) {
        self.repos_to_sync.push(repo.to_owned());
    }

    pub async fn run(&self, duration_seconds: u64) -> anyhow::Result<()> {
        loop {
            self.do_sync().await?;
            tokio::time::sleep(std::time::Duration::from_secs(duration_seconds)).await;
        }
    }

    async fn do_sync(&self) -> anyhow::Result<()> {
        let mut stats = self.read_persisted_stats().unwrap_or_else(|e| {
            warn!("Failed to read persisted stats, using empty instead: {}", e);
            GitHubRepoStats::default()
        });
        for repo in &self.repos_to_sync {
            let clones = self
                .client
                .get_traffic_clones_per_day(repo.as_str())
                .await?;
            let views = self.client.get_traffic_views_per_day(repo.as_str()).await?;

            let repo_stats = stats.entry(repo.clone()).or_default();
            find_and_insert(&mut repo_stats.clones, clones.clones, |a, b| {
                a.timestamp.cmp(&b.timestamp)
            });
            find_and_insert(&mut repo_stats.views, views.views, |a, b| {
                a.timestamp.cmp(&b.timestamp)
            });
        }
        self.write_persisted_stats(&stats)?;

        Ok(())
    }

    fn read_persisted_stats(&self) -> anyhow::Result<GitHubRepoStats> {
        read_persisted_stats(self.db_file.as_str())
    }

    fn write_persisted_stats(&self, stats: &GitHubRepoStats) -> anyhow::Result<()> {
        let content = serde_json::to_string(stats)?;
        std::fs::write(&self.db_file, content)?;
        Ok(())
    }
}

pub struct GitHubClient {
    pub username: String,
    pub access_token: String,
}

impl GitHubClient {
    pub fn new(username: &str, access_token: &str) -> Self {
        Self {
            username: username.to_owned(),
            access_token: access_token.to_owned(),
        }
    }

    pub async fn get_traffic_clones_per_day(
        &self,
        repo: &str,
    ) -> anyhow::Result<GitHubMetricTrafficClones> {
        self.get_traffic_info("clones", repo).await
    }

    pub async fn get_traffic_views_per_day(
        &self,
        repo: &str,
    ) -> anyhow::Result<GitHubMetricTrafficViews> {
        self.get_traffic_info("views", repo).await
    }

    async fn get_traffic_info<T: DeserializeOwned>(
        &self,
        path: &str,
        repo: &str,
    ) -> anyhow::Result<T> {
        let client = reqwest::Client::new();
        let res = client
            .get(format!(
                "https://api.github.com/repos/{}/traffic/{}",
                repo, path,
            ))
            .header("User-Agent", format!("github-metrics v{}", VERSION))
            .query(&[("per", "day")])
            .basic_auth(&self.username, Some(&self.access_token))
            .send()
            .await?;

        if res.status().as_u16() == 403 {
            return Err(GitHubAccessError::AccessForbidden.into());
        }

        Ok(res.json().await?)
    }

    pub async fn list_user_repos<T: DeserializeOwned>(&self) -> anyhow::Result<T> {
        let client = reqwest::Client::new();
        let res = client
            .get(format!(
                "https://api.github.com/users/{}/repos",
                self.username,
            ))
            .header("User-Agent", format!("github-metrics v{}", VERSION))
            .basic_auth(&self.username, Some(&self.access_token))
            .send()
            .await?;

        if res.status().as_u16() == 403 {
            return Err(GitHubAccessError::AccessForbidden.into());
        }

        Ok(res.json().await?)
    }
}

#[derive(Debug)]
pub enum GitHubAccessError {
    AccessForbidden,
}

impl std::fmt::Display for GitHubAccessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "403 Access Forbidden")
    }
}

impl std::error::Error for GitHubAccessError {}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubRepo {
    pub id: i64,
    pub name: String,
    #[serde(rename = "full_name")]
    pub full_name: String,
    pub private: bool,
    pub fork: bool,
}

pub type GitHubRepoStats = HashMap<String, GitHubRepoTraffics>;

#[derive(Default, Debug, Clone, Serialize, Deserialize, Object)]
pub struct GitHubRepoTraffics {
    pub clones: Vec<GitHubMetricTrafficItem>,
    pub views: Vec<GitHubMetricTrafficItem>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct GitHubMetricTrafficClones {
    pub count: i64,
    pub uniques: i64,
    pub clones: Vec<GitHubMetricTrafficItem>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct GitHubMetricTrafficViews {
    pub count: i64,
    pub uniques: i64,
    pub views: Vec<GitHubMetricTrafficItem>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, Object)]
pub struct GitHubMetricTrafficItem {
    pub timestamp: String,
    pub count: i64,
    pub uniques: i64,
}

fn find_and_insert<T>(target: &mut Vec<T>, source: Vec<T>, cmp: impl Fn(&T, &T) -> Ordering) {
    if target.len() == 0 {
        *target = source;
        return;
    }
    for src_item in source {
        if let Some(found) = target
            .iter_mut()
            .find(|c| cmp(c, &src_item) == Ordering::Equal)
        {
            *found = src_item;
        } else {
            let target_count = target.len();
            for i in 0..(target_count + 1) {
                if let Some(found) = target.get(i) {
                    if cmp(found, &src_item) == Ordering::Greater {
                        target.insert(i, src_item);
                        break;
                    }
                } else {
                    target.push(src_item);
                    break;
                }
            }
        }
    }
}

pub fn read_persisted_stats(db_file: &str) -> anyhow::Result<GitHubRepoStats> {
    let content = match std::fs::read_to_string(db_file) {
        Ok(c) => c,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(GitHubRepoStats::default());
        }
        Err(err) => return Err(err.into()),
    };
    let stats: GitHubRepoStats = serde_json::from_str(&content)?;
    Ok(stats)
}

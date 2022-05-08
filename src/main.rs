use github::{GitHubClient, GitHubTrafficsSync};
use log::{info, LevelFilter};
use simple_logger::SimpleLogger;

use crate::version::VERSION;

mod github;
mod version;

#[tokio::main]
async fn main() {
    SimpleLogger::new()
        .with_level(LevelFilter::Debug)
        .init()
        .unwrap();

    info!("GitHub Metrics Sync v{}", VERSION);

    let username = std::env::var("GITHUB_USERNAME").expect("请设置环境变量 GITHUB_USERNAME");
    let access_token =
        std::env::var("GITHUB_ACCESS_TOKEN").expect("请设置环境变量 GITHUB_ACCESS_TOKEN");
    let repo_list_str = std::env::var("GITHUB_REPOS").expect("请设置环境变量 GITHUB_REPOS");

    let duration_seconds = std::env::var("GITHUB_SYNC_DURATION")
        .ok()
        .and_then(|dur| dur.parse().ok())
        .unwrap_or(6 * 60 * 60);

    let client = GitHubClient::new(username.as_str(), access_token.as_str());
    let mut sync = GitHubTrafficsSync::new(client, "traffics.json".to_owned());
    for repo in repo_list_str.split(":") {
        sync.add_repo(repo);
    }
    sync.run(duration_seconds).await.unwrap();
}

use runner_mgr::github::GitHubClient;

#[tokio::test]
async fn test_client_creation() {
    let client = GitHubClient::new("ghp_fake_token");
    let _ = client;
}

#[tokio::test]
async fn test_invalid_token_returns_error() {
    let client = GitHubClient::new("ghp_definitely_not_a_real_token");
    let result = client.get_user().await;
    assert!(result.is_err(), "invalid token should return an error");
}

#[tokio::test]
async fn test_list_repos_invalid_token() {
    let client = GitHubClient::new("ghp_invalid");
    let result = client.list_repos().await;
    assert!(
        result.is_err(),
        "listing repos with invalid token should fail"
    );
}

#[tokio::test]
async fn test_registration_token_invalid_repo() {
    let client = GitHubClient::new("ghp_invalid");
    let result = client.get_registration_token("nonexistent/repo").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_list_runners_invalid_token() {
    let client = GitHubClient::new("ghp_invalid");
    let result = client.list_runners("nonexistent/repo").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_list_workflow_runs_invalid_token() {
    let client = GitHubClient::new("ghp_invalid");
    let result = client.list_workflow_runs("nonexistent/repo", 5).await;
    assert!(result.is_err());
}

use serial_test::serial;
use tempfile::TempDir;

#[test]
fn test_list_instances_empty() {
    let tmp = TempDir::new().unwrap();
    let config = runner_mgr::config::Config {
        github_pat: "ghp_test".to_string(),
        github_user: "user".to_string(),
        runner_user: "github".to_string(),
        runner_os: "linux".to_string(),
        runner_arch: "x64".to_string(),
        instances_base: tmp.path().to_str().unwrap().to_string(),
    };

    let instances = runner_mgr::runner::list_instances(&config);
    assert!(
        instances.is_empty(),
        "should return empty list when no instances dir"
    );
}

#[test]
fn test_list_instances_with_dirs() {
    let tmp = TempDir::new().unwrap();
    let instances_dir = tmp.path().join("instances");
    std::fs::create_dir_all(instances_dir.join("owner__repo1")).unwrap();
    std::fs::create_dir_all(instances_dir.join("owner__repo2")).unwrap();

    let config = runner_mgr::config::Config {
        github_pat: "ghp_test".to_string(),
        github_user: "user".to_string(),
        runner_user: "github".to_string(),
        runner_os: "linux".to_string(),
        runner_arch: "x64".to_string(),
        instances_base: tmp.path().to_str().unwrap().to_string(),
    };

    let instances = runner_mgr::runner::list_instances(&config);
    assert_eq!(instances.len(), 2);

    let repos: Vec<&str> = instances.iter().map(|i| i.repo.as_str()).collect();
    assert!(repos.contains(&"owner/repo1"));
    assert!(repos.contains(&"owner/repo2"));
}

#[test]
fn test_list_instances_sorted() {
    let tmp = TempDir::new().unwrap();
    let instances_dir = tmp.path().join("instances");
    std::fs::create_dir_all(instances_dir.join("zzz__repo")).unwrap();
    std::fs::create_dir_all(instances_dir.join("aaa__repo")).unwrap();
    std::fs::create_dir_all(instances_dir.join("mmm__repo")).unwrap();

    let config = runner_mgr::config::Config {
        github_pat: "ghp_test".to_string(),
        github_user: "user".to_string(),
        runner_user: "github".to_string(),
        runner_os: "linux".to_string(),
        runner_arch: "x64".to_string(),
        instances_base: tmp.path().to_str().unwrap().to_string(),
    };

    let instances = runner_mgr::runner::list_instances(&config);
    assert_eq!(instances.len(), 3);
    assert_eq!(instances[0].repo, "aaa/repo");
    assert_eq!(instances[1].repo, "mmm/repo");
    assert_eq!(instances[2].repo, "zzz/repo");
}

#[test]
fn test_runner_status_display() {
    assert_eq!(
        format!("{}", runner_mgr::runner::RunnerStatus::Running),
        "running"
    );
    assert_eq!(
        format!("{}", runner_mgr::runner::RunnerStatus::Stopped),
        "stopped"
    );
    assert_eq!(
        format!("{}", runner_mgr::runner::RunnerStatus::NoService),
        "no service"
    );
    assert_eq!(
        format!("{}", runner_mgr::runner::RunnerStatus::Unknown),
        "unknown"
    );
}

#[test]
fn test_instance_with_service_file() {
    let tmp = TempDir::new().unwrap();
    let instances_dir = tmp.path().join("instances");
    let repo_dir = instances_dir.join("owner__repo1");
    std::fs::create_dir_all(&repo_dir).unwrap();
    std::fs::write(repo_dir.join(".service"), "actions.runner.myservice").unwrap();

    let config = runner_mgr::config::Config {
        github_pat: "ghp_test".to_string(),
        github_user: "user".to_string(),
        runner_user: "github".to_string(),
        runner_os: "linux".to_string(),
        runner_arch: "x64".to_string(),
        instances_base: tmp.path().to_str().unwrap().to_string(),
    };

    let instances = runner_mgr::runner::list_instances(&config);
    assert_eq!(instances.len(), 1);
    assert_eq!(
        instances[0].service_name.as_deref(),
        Some("actions.runner.myservice")
    );
}

#[test]
fn test_get_logs_nonexistent_repo() {
    let tmp = TempDir::new().unwrap();
    let config = runner_mgr::config::Config {
        github_pat: "ghp_test".to_string(),
        github_user: "user".to_string(),
        runner_user: "github".to_string(),
        runner_os: "linux".to_string(),
        runner_arch: "x64".to_string(),
        instances_base: tmp.path().to_str().unwrap().to_string(),
    };

    let result = runner_mgr::runner::get_runner_logs(&config, "nonexistent/repo", 50);
    assert!(result.is_err());
    let err = format!("{:#}", result.unwrap_err());
    assert!(err.contains("No runner configured"));
}

// Tests for import functionality

#[test]
fn test_parse_repo_from_runner_config_valid() {
    let content = r#"{"gitHubUrl": "https://github.com/myowner/myrepo"}"#;
    let result = runner_mgr::runner::parse_repo_from_runner_config(content).unwrap();
    assert_eq!(result, "myowner/myrepo");
}

#[test]
fn test_parse_repo_from_runner_config_with_trailing_slash() {
    let content = r#"{"gitHubUrl": "https://github.com/owner/repo/"}"#;
    let result = runner_mgr::runner::parse_repo_from_runner_config(content).unwrap();
    assert_eq!(result, "owner/repo");
}

#[test]
fn test_parse_repo_from_runner_config_http_url() {
    let content = r#"{"gitHubUrl": "http://github.com/owner/repo"}"#;
    let result = runner_mgr::runner::parse_repo_from_runner_config(content).unwrap();
    assert_eq!(result, "owner/repo");
}

#[test]
fn test_parse_repo_from_runner_config_missing_url() {
    let content = r#"{"somethingElse": "value"}"#;
    let result = runner_mgr::runner::parse_repo_from_runner_config(content);
    assert!(result.is_err());
    let err = format!("{:#}", result.unwrap_err());
    assert!(err.contains("No gitHubUrl found"));
}

#[test]
fn test_parse_repo_from_runner_config_invalid_json() {
    let content = "not valid json";
    let result = runner_mgr::runner::parse_repo_from_runner_config(content);
    assert!(result.is_err());
    let err = format!("{:#}", result.unwrap_err());
    assert!(err.contains("Failed to parse"));
}

#[test]
fn test_parse_repo_from_runner_config_unexpected_format() {
    let content = r#"{"gitHubUrl": "https://gitlab.com/owner/repo"}"#;
    let result = runner_mgr::runner::parse_repo_from_runner_config(content);
    assert!(result.is_err());
    let err = format!("{:#}", result.unwrap_err());
    assert!(err.contains("Unexpected gitHubUrl format"));
}

#[test]
#[serial]
fn test_import_runner_nonexistent_path() {
    let tmp = TempDir::new().unwrap();
    std::env::set_var("RUNNER_MGR_CONFIG_DIR", tmp.path().join("config"));

    let config = runner_mgr::config::Config {
        github_pat: "ghp_test".to_string(),
        github_user: "user".to_string(),
        runner_user: "github".to_string(),
        runner_os: "darwin".to_string(),
        runner_arch: "arm64".to_string(),
        instances_base: tmp.path().join("runners").to_str().unwrap().to_string(),
    };
    config.save().unwrap();

    let result = runner_mgr::runner::import_runner(&config, "/nonexistent/path", None);
    assert!(result.is_err());
    let err = format!("{:#}", result.unwrap_err());
    assert!(err.contains("does not exist"));

    std::env::remove_var("RUNNER_MGR_CONFIG_DIR");
}

#[test]
#[serial]
fn test_import_runner_not_a_runner_directory() {
    let tmp = TempDir::new().unwrap();
    let fake_runner = tmp.path().join("fake-runner");
    std::fs::create_dir_all(&fake_runner).unwrap();
    // Missing config.sh - not a valid runner directory

    std::env::set_var("RUNNER_MGR_CONFIG_DIR", tmp.path().join("config"));

    let config = runner_mgr::config::Config {
        github_pat: "ghp_test".to_string(),
        github_user: "user".to_string(),
        runner_user: "github".to_string(),
        runner_os: "darwin".to_string(),
        runner_arch: "arm64".to_string(),
        instances_base: tmp.path().join("runners").to_str().unwrap().to_string(),
    };
    config.save().unwrap();

    let result = runner_mgr::runner::import_runner(&config, fake_runner.to_str().unwrap(), None);
    assert!(result.is_err());
    let err = format!("{:#}", result.unwrap_err());
    assert!(err.contains("Not a valid runner directory"));

    std::env::remove_var("RUNNER_MGR_CONFIG_DIR");
}

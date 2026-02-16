use runner_mgr::github::RunnerScope;

// Tests for RunnerScope::parse()

#[test]
fn test_parse_repository_scope() {
    let scope = RunnerScope::parse("owner/repo").unwrap();
    assert!(matches!(
        scope,
        RunnerScope::Repository { owner, repo } if owner == "owner" && repo == "repo"
    ));
}

#[test]
fn test_parse_organization_scope() {
    let scope = RunnerScope::parse("org:myorg").unwrap();
    assert!(matches!(
        scope,
        RunnerScope::Organization { org } if org == "myorg"
    ));
}

#[test]
fn test_parse_invalid_format() {
    let result = RunnerScope::parse("invalid");
    assert!(result.is_err());
    let err = format!("{:#}", result.unwrap_err());
    assert!(err.contains("Invalid identifier"));
}

#[test]
fn test_parse_empty_org_name() {
    let result = RunnerScope::parse("org:");
    assert!(result.is_err());
    let err = format!("{:#}", result.unwrap_err());
    assert!(err.contains("empty"));
}

#[test]
fn test_parse_org_with_slash() {
    let result = RunnerScope::parse("org:my/org");
    assert!(result.is_err());
    let err = format!("{:#}", result.unwrap_err());
    assert!(err.contains("cannot contain"));
}

#[test]
fn test_parse_repo_missing_owner() {
    let result = RunnerScope::parse("/repo");
    assert!(result.is_err());
    let err = format!("{:#}", result.unwrap_err());
    assert!(err.contains("owner/repo"));
}

#[test]
fn test_parse_repo_missing_repo() {
    let result = RunnerScope::parse("owner/");
    assert!(result.is_err());
    let err = format!("{:#}", result.unwrap_err());
    assert!(err.contains("owner/repo"));
}

// Tests for to_dir_name() and from_dir_name() roundtrip

#[test]
fn test_repository_dir_name_roundtrip() {
    let original = RunnerScope::Repository {
        owner: "myowner".to_string(),
        repo: "myrepo".to_string(),
    };
    let dir_name = original.to_dir_name();
    assert_eq!(dir_name, "myowner__myrepo");

    let parsed = RunnerScope::from_dir_name(&dir_name).unwrap();
    assert_eq!(original, parsed);
}

#[test]
fn test_organization_dir_name_roundtrip() {
    let original = RunnerScope::Organization {
        org: "myorg".to_string(),
    };
    let dir_name = original.to_dir_name();
    assert_eq!(dir_name, "org__myorg");

    let parsed = RunnerScope::from_dir_name(&dir_name).unwrap();
    assert_eq!(original, parsed);
}

#[test]
fn test_from_dir_name_invalid() {
    assert!(RunnerScope::from_dir_name("invalid").is_none());
    assert!(RunnerScope::from_dir_name("").is_none());
    assert!(RunnerScope::from_dir_name("__").is_none());
    assert!(RunnerScope::from_dir_name("org__").is_none());
}

// Tests for from_github_url()

#[test]
fn test_from_github_url_repository() {
    let scope = RunnerScope::from_github_url("https://github.com/owner/repo").unwrap();
    assert!(matches!(
        scope,
        RunnerScope::Repository { owner, repo } if owner == "owner" && repo == "repo"
    ));
}

#[test]
fn test_from_github_url_repository_trailing_slash() {
    let scope = RunnerScope::from_github_url("https://github.com/owner/repo/").unwrap();
    assert!(matches!(
        scope,
        RunnerScope::Repository { owner, repo } if owner == "owner" && repo == "repo"
    ));
}

#[test]
fn test_from_github_url_organization() {
    let scope = RunnerScope::from_github_url("https://github.com/myorg").unwrap();
    assert!(matches!(
        scope,
        RunnerScope::Organization { org } if org == "myorg"
    ));
}

#[test]
fn test_from_github_url_organization_trailing_slash() {
    let scope = RunnerScope::from_github_url("https://github.com/myorg/").unwrap();
    assert!(matches!(
        scope,
        RunnerScope::Organization { org } if org == "myorg"
    ));
}

#[test]
fn test_from_github_url_http() {
    let scope = RunnerScope::from_github_url("http://github.com/owner/repo").unwrap();
    assert!(matches!(
        scope,
        RunnerScope::Repository { owner, repo } if owner == "owner" && repo == "repo"
    ));
}

#[test]
fn test_from_github_url_invalid() {
    let result = RunnerScope::from_github_url("https://gitlab.com/owner/repo");
    assert!(result.is_err());
    let err = format!("{:#}", result.unwrap_err());
    assert!(err.contains("Unexpected"));
}

// Tests for to_display()

#[test]
fn test_repository_display() {
    let scope = RunnerScope::Repository {
        owner: "owner".to_string(),
        repo: "repo".to_string(),
    };
    assert_eq!(scope.to_display(), "owner/repo");
}

#[test]
fn test_organization_display() {
    let scope = RunnerScope::Organization {
        org: "myorg".to_string(),
    };
    assert_eq!(scope.to_display(), "org:myorg");
}

// Tests for github_url()

#[test]
fn test_repository_github_url() {
    let scope = RunnerScope::Repository {
        owner: "owner".to_string(),
        repo: "repo".to_string(),
    };
    assert_eq!(scope.github_url(), "https://github.com/owner/repo");
}

#[test]
fn test_organization_github_url() {
    let scope = RunnerScope::Organization {
        org: "myorg".to_string(),
    };
    assert_eq!(scope.github_url(), "https://github.com/myorg");
}

// Tests for supports_workflow_runs()

#[test]
fn test_repository_supports_workflow_runs() {
    let scope = RunnerScope::Repository {
        owner: "owner".to_string(),
        repo: "repo".to_string(),
    };
    assert!(scope.supports_workflow_runs());
}

#[test]
fn test_organization_does_not_support_workflow_runs() {
    let scope = RunnerScope::Organization {
        org: "myorg".to_string(),
    };
    assert!(!scope.supports_workflow_runs());
}

// Tests for api_path()

#[test]
fn test_repository_api_path() {
    let scope = RunnerScope::Repository {
        owner: "owner".to_string(),
        repo: "repo".to_string(),
    };
    assert_eq!(scope.api_path(), "repos/owner/repo");
}

#[test]
fn test_organization_api_path() {
    let scope = RunnerScope::Organization {
        org: "myorg".to_string(),
    };
    assert_eq!(scope.api_path(), "orgs/myorg");
}

// Tests for Hash and Eq implementations

#[test]
fn test_scope_equality() {
    let scope1 = RunnerScope::Repository {
        owner: "owner".to_string(),
        repo: "repo".to_string(),
    };
    let scope2 = RunnerScope::Repository {
        owner: "owner".to_string(),
        repo: "repo".to_string(),
    };
    assert_eq!(scope1, scope2);
}

#[test]
fn test_scope_inequality() {
    let repo_scope = RunnerScope::Repository {
        owner: "owner".to_string(),
        repo: "repo".to_string(),
    };
    let org_scope = RunnerScope::Organization {
        org: "owner".to_string(),
    };
    assert_ne!(repo_scope, org_scope);
}

#[test]
fn test_scope_hash() {
    use std::collections::HashSet;

    let mut set = HashSet::new();
    let scope1 = RunnerScope::Repository {
        owner: "owner".to_string(),
        repo: "repo".to_string(),
    };
    let scope2 = RunnerScope::Repository {
        owner: "owner".to_string(),
        repo: "repo".to_string(),
    };

    set.insert(scope1);
    assert!(set.contains(&scope2));
}

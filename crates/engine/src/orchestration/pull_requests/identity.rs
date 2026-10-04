//! T3 host-level identities. A URL wins over every other target field.
use reqwest::Url;
use serde::{Deserialize, Serialize};

use super::PrError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Identity {
    pub host: String,
    pub repository: String,
    pub number: i64,
    pub url: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Target {
    pub url: Option<String>,
    pub repository: Option<String>,
    pub number: Option<i64>,
    pub host: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProjectHost {
    pub host: String,
    pub kind: String,
    pub remote_url: String,
}

fn is_host(host: &str, apex: &str, label: &str) -> bool {
    host == apex || host.ends_with(&format!(".{apex}")) || host.split('.').any(|s| s == label)
}

pub fn canonical(host: &str, repository: &str) -> (String, String) {
    let host = host.trim().to_lowercase();
    let repository = repository.trim().to_lowercase();
    let parts: Vec<_> = repository.split('/').collect();
    if matches!(
        host.as_str(),
        "ssh.dev.azure.com" | "vs-ssh.visualstudio.com"
    ) && parts.len() == 4
        && parts[0] == "v3"
    {
        return (
            "dev.azure.com".into(),
            format!("{}/{}/_git/{}", parts[1], parts[2], parts[3]),
        );
    }
    if let Some(org) = host.strip_suffix(".visualstudio.com")
        && !org.contains('.')
    {
        let path = repository
            .strip_prefix("defaultcollection/")
            .unwrap_or(&repository);
        let parts: Vec<_> = path.split('/').collect();
        if parts.len() == 3 && parts[1] == "_git" {
            return ("dev.azure.com".into(), format!("{org}/{path}"));
        }
    }
    (host, repository)
}

pub fn parse_url(value: &str) -> Option<Identity> {
    let url = Url::parse(value).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    let host = url.host_str()?.to_lowercase();
    static ROUTES: std::sync::OnceLock<[regex::Regex; 5]> = std::sync::OnceLock::new();
    let routes = ROUTES.get_or_init(|| {
        [
            r"^/([^/]+/[^/]+)/pull/(\d+)(?:/|$)",
            r"^/([^/]+(?:/[^/]+)+)/pulls/(\d+)(?:/|$)",
            r"^/([^/]+(?:/[^/]+)+)/-/merge_requests/(\d+)(?:/|$)",
            r"^/([^/]+/[^/]+)/pull-requests/(\d+)(?:/|$)",
            r"^/((?:[^/]+/)*_git/[^/]+)/pullrequest/(\d+)(?:/|$)",
        ]
        .map(|s| regex::Regex::new(s).expect("constant PR route"))
    });
    let (index, matched) = routes.iter().enumerate().find_map(|(index, route)| {
        let allowed = match index {
            0 => is_host(&host, "github.com", "github"),
            3 => is_host(&host, "bitbucket.org", "bitbucket"),
            4 => is_host(&host, "dev.azure.com", "") || host.ends_with(".visualstudio.com"),
            _ => true,
        };
        allowed
            .then(|| route.captures(url.path()))
            .flatten()
            .map(|c| (index, c))
    })?;
    let number_text = matched.get(2)?.as_str();
    let number = number_text.parse::<i64>().ok()?;
    if !(1..=9_007_199_254_740_991).contains(&number) {
        return None;
    }
    let repository = matched.get(1)?.as_str();
    let authority = if index == 1 {
        url.port()
            .map_or(host.clone(), |port| format!("{host}:{port}"))
    } else {
        host
    };
    let (host, repository) = canonical(&authority, repository);
    Some(Identity {
        host,
        repository,
        number,
        url: value.into(),
    })
}

impl Identity {
    pub fn key(&self) -> String {
        format!("{}/{}#{}", self.host, self.repository, self.number)
    }
}

pub fn resolve(input: Target, project: Option<&ProjectHost>) -> Result<Identity, PrError> {
    if let Some(url) = input.url {
        return parse_url(&url).ok_or_else(|| PrError::new("PullRequestUrlInvalidError"));
    }
    let (Some(repository), Some(number)) = (input.repository, input.number) else {
        return Err(PrError::new("PullRequestTargetIncompleteError"));
    };
    let host = input
        .host
        .or_else(|| project.map(|p| p.host.clone()))
        .ok_or_else(|| PrError::new("PullRequestHostRequiredError"))?
        .to_lowercase();
    let repository = repository.to_lowercase();
    let kind = project.filter(|p| p.host == host).map(|p| p.kind.as_str());
    let path = match kind {
        Some("gitlab") => format!("{repository}/-/merge_requests/{number}"),
        Some("forgejo") => format!("{repository}/pulls/{number}"),
        Some("bitbucket") => format!("{repository}/pull-requests/{number}"),
        Some("azure-devops") => {
            let (host, repository) = canonical(&host, &repository);
            return Ok(Identity {
                url: format!("https://{host}/{repository}/pullrequest/{number}"),
                host,
                repository,
                number,
            });
        }
        _ => format!("{repository}/pull/{number}"),
    };
    let origin = project
        .filter(|p| kind == Some("forgejo") && p.host == host)
        .and_then(|p| Url::parse(&p.remote_url).ok())
        .filter(|u| matches!(u.scheme(), "http" | "https"))
        .map(|u| u.origin().ascii_serialization())
        .unwrap_or_else(|| format!("https://{host}"));
    let url = format!("{origin}/{path}");
    let (host, repository) = parse_url(&url)
        .map(|i| (i.host, i.repository))
        .unwrap_or_else(|| canonical(&host, &repository));
    Ok(Identity {
        host,
        repository,
        number,
        url,
    })
}

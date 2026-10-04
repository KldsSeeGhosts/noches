use super::identity::{Identity, canonical, parse_url};
use std::collections::{HashMap, HashSet};
use zeron_proto::orchestration::{ThreadPullRequestLink, ThreadPullRequestLinkSource};

pub fn identity(link: &ThreadPullRequestLink) -> Identity {
    let parsed = parse_url(&link.url).filter(|p| {
        p.repository == link.repository.trim().to_lowercase() && p.number == link.number
    });
    let (host, repository) = canonical(
        parsed.as_ref().map_or(&link.host, |p| &p.host),
        &link.repository,
    );
    Identity {
        host,
        repository,
        number: link.number,
        url: link.url.clone(),
    }
}

pub fn visible(link: &&ThreadPullRequestLink) -> bool {
    link.source != ThreadPullRequestLinkSource::StackDismissed
}

pub struct Chain<'a> {
    pub kind: &'static str,
    pub layers: Vec<&'a ThreadPullRequestLink>,
}

pub fn resolve(links: &[ThreadPullRequestLink]) -> Vec<Chain<'_>> {
    let visible: Vec<_> = links.iter().filter(visible).collect();
    let mut chains = vec![];
    let mut placed = HashSet::new();
    // Vec preserves insertion order, unlike a randomized HashMap.
    let mut native: Vec<(String, Vec<&ThreadPullRequestLink>)> = vec![];
    for link in &visible {
        if let Some(stack) = &link.stack {
            let id = identity(link);
            let key = format!("{}/{}#stack:{}", id.host, id.repository, stack.id);
            if let Some((_, members)) = native.iter_mut().find(|(k, _)| k == &key) {
                members.push(link);
            } else {
                native.push((key, vec![link]));
            }
        }
    }
    for (_, mut members) in native {
        let order: Vec<_> = members[0]
            .stack
            .as_ref()
            .unwrap()
            .layers
            .iter()
            .map(|l| l.number)
            .collect();
        members.sort_by_key(|l| order.iter().position(|n| *n == l.number).unwrap_or(0));
        placed.extend(members.iter().map(|l| identity(l).key()));
        chains.push(Chain {
            kind: "native",
            layers: members,
        });
    }
    let remaining: Vec<_> = visible
        .into_iter()
        .filter(|l| !placed.contains(&identity(l).key()))
        .collect();
    let branch_key = |link: &ThreadPullRequestLink, branch: &str| {
        let id = identity(link);
        format!("{}/{}:{branch}", id.host, id.repository)
    };
    let mut heads: HashMap<String, Option<&ThreadPullRequestLink>> = HashMap::new();
    for link in &remaining {
        if let Some(snapshot) = &link.snapshot {
            let key = branch_key(link, &snapshot.head_branch);
            heads
                .entry(key)
                .and_modify(|v| *v = None)
                .or_insert(Some(link));
        }
    }
    let parent = |link: &ThreadPullRequestLink| {
        link.snapshot
            .as_ref()
            .and_then(|s| heads.get(&branch_key(link, &s.base_branch)))
            .copied()
            .flatten()
    };
    let mut has_child = HashSet::new();
    for link in &remaining {
        if let Some(p) = parent(link)
            && identity(p).key() != identity(link).key()
        {
            has_child.insert(identity(p).key());
        }
    }
    for top in &remaining {
        if has_child.contains(&identity(top).key()) {
            continue;
        }
        let mut layers = vec![];
        let mut cursor = Some(*top);
        while let Some(link) = cursor {
            if !placed.insert(identity(link).key()) {
                break;
            }
            layers.insert(0, link);
            cursor = parent(link);
        }
        if !layers.is_empty() {
            chains.push(Chain {
                kind: "derived",
                layers,
            });
        }
    }
    for link in remaining {
        if !placed.contains(&identity(link).key()) {
            chains.push(Chain {
                kind: "derived",
                layers: vec![link],
            });
        }
    }
    chains
}

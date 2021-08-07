use crate::utils::{CmError, DynError, Severity};
use git2::Repository;

pub enum PullResult {
	Clean,
	Conflicted { conflicted_files: usize },
	NothingToDo,
}

use std::io::{self, Write};
use std::str;

// forked from git2-rs examples
fn do_fetch<'a>(
	repo: &'a git2::Repository,
	tags: &[&str],
	caf_path: &str,
) -> Result<git2::AnnotatedCommit<'a>, DynError> {
	let mut cb = git2::RemoteCallbacks::new();
	let remote_url = crate::CAF_BASE_URL.to_string() + caf_path;

	let mut remote = repo
		.find_remote(&remote_url)
		.or_else(|_| repo.remote_anonymous(&remote_url))?;

	cb.transfer_progress(|stats| {
		if stats.received_objects() == stats.total_objects() {
			print!(
				"Resolving deltas {}/{}\r",
				stats.indexed_deltas(),
				stats.total_deltas()
			);
		} else if stats.total_objects() > 0 {
			print!(
				"Received {}/{} objects ({}) in {} bytes\r",
				stats.received_objects(),
				stats.total_objects(),
				stats.indexed_objects(),
				stats.received_bytes()
			);
		}
		io::stdout().flush().unwrap();
		true
	});

	let mut fo = git2::FetchOptions::new();
	fo.remote_callbacks(cb);
	fo.download_tags(git2::AutotagOption::Auto);
	remote.fetch(tags, Some(&mut fo), None)?;

	// If there are local objects (we got a thin pack), then tell the user
	// how many objects we saved from having to cross the network.
	let stats = remote.stats();
	if stats.local_objects() > 0 {
		println!(
			"\rReceived {}/{} objects in {} bytes (used {} local \
             objects)",
			stats.indexed_objects(),
			stats.total_objects(),
			stats.received_bytes(),
			stats.local_objects()
		);
	} else {
		println!(
			"\rReceived {}/{} objects in {} bytes",
			stats.indexed_objects(),
			stats.total_objects(),
			stats.received_bytes()
		);
	}

	let fetch_head = repo.find_reference("FETCH_HEAD")?;
	Ok(repo.reference_to_annotated_commit(&fetch_head)?)
}

// forked from git2-rs examples
fn normal_merge(
	repo: &Repository,
	local: &git2::AnnotatedCommit,
	remote: &git2::AnnotatedCommit,
	tag: String,
	caf_path: String,
) -> Result<PullResult, git2::Error> {
	let local_tree = repo.find_commit(local.id())?.tree()?;
	let remote_tree = repo.find_commit(remote.id())?.tree()?;
	let ancestor = repo
		.find_commit(repo.merge_base(local.id(), remote.id())?)?
		.tree()?;
	let mut idx = repo.merge_trees(&ancestor, &local_tree, &remote_tree, None)?;

	if idx.has_conflicts() {
		repo.checkout_index(Some(&mut idx), None)?;
		return Ok(PullResult::Conflicted {
			conflicted_files: idx.conflicts()?.count(),
		});
	}
	let result_tree = repo.find_tree(idx.write_tree_to(repo)?)?;
	// now create the merge commit
	let msg = format!(
		"Merge tag '{}' of {}{} into HEAD",
		tag,
		crate::CAF_BASE_URL,
		caf_path
	);
	let sig = repo.signature()?;
	let local_commit = repo.find_commit(local.id())?;
	let remote_commit = repo.find_commit(remote.id())?;
	// Do our merge commit and set current branch head to that commit.
	let _merge_commit = repo.commit(
		Some("HEAD"),
		&sig,
		&sig,
		&msg,
		&result_tree,
		&[&local_commit, &remote_commit],
	)?;
	// Set working tree to match head.
	repo.checkout_head(None)?;
	Ok(PullResult::Clean)
}

pub fn pull(git_path: &str, caf_path: &str, tag: String) -> Result<PullResult, DynError> {
	let repo = match Repository::open(git_path) {
		Ok(r) => r,
		Err(e) => {
			return Err(Box::new(CmError {
				severity: Severity::Warning,
				message: format!("couldn't find a git repo in '{}': {}", git_path, e),
			}))
		}
	};

	let fetch_commit = do_fetch(&repo, &[tag.as_str()], caf_path)?;
	let analysis = repo.merge_analysis(&[&fetch_commit])?;

	if analysis.0.is_normal() || analysis.0.is_fast_forward() {
		let head_commit = repo.reference_to_annotated_commit(&repo.head()?)?;
		Ok(normal_merge(
			&repo,
			&head_commit,
			&fetch_commit,
			tag,
			caf_path.to_string(),
		)?)
	} else {
		Ok(PullResult::NothingToDo)
	}
}

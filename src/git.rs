use crate::utils::{CmError, DynError, Severity};
use git2::Repository;

pub enum GitResult {
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

	cb.sideband_progress(|data| {
		print!("remote: {}", str::from_utf8(data).unwrap());
		io::stdout().flush().unwrap();
		true
	});

	// This callback gets called for each remote-tracking branch that gets
	// updated. The message we output depends on whether it's a new one or an
	// update.
	cb.update_tips(|refname, a, b| {
		if a.is_zero() {
			println!("[new]     {:20} {}", b, refname);
		} else {
			println!("[updated] {:10}..{:10} {}", a, b, refname);
		}
		true
	});

	cb.transfer_progress(|stats| {
		if stats.received_objects() == stats.total_objects() {
			print!(
				"{}Resolving deltas {}/{}\r",
				termion::clear::CurrentLine,
				stats.indexed_deltas(),
				stats.total_deltas()
			);
		} else if stats.total_objects() > 0 {
			print!(
				"{}Received {}/{} objects ({}) in {} bytes\r",
				termion::clear::CurrentLine,
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
	remote.download(tags, Some(&mut fo))?;

	{
		// If there are local objects (we got a thin pack), then tell the user
		// how many objects we saved from having to cross the network.
		let stats = remote.stats();
		if stats.local_objects() > 0 {
			println!(
				"{}\rReceived {}/{} objects in {} bytes (used {} local \
						 objects)",
				termion::clear::CurrentLine,
				stats.indexed_objects(),
				stats.total_objects(),
				stats.received_bytes(),
				stats.local_objects()
			);
		} else {
			println!(
				"{}\rReceived {}/{} objects in {} bytes",
				termion::clear::CurrentLine,
				stats.indexed_objects(),
				stats.total_objects(),
				stats.received_bytes()
			);
		}
	}

	// Disconnect the underlying connection to prevent from idling.
	remote.disconnect()?;

	// Update the references in the remote's namespace to point to the right
	// commits. This may be needed even if there was no packfile to download,
	// which can happen e.g. when the branches have been changed but all the
	// needed objects are available locally.
	remote.update_tips(None, true, git2::AutotagOption::Unspecified, None)?;

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
) -> Result<GitResult, git2::Error> {
	let local_tree = repo.find_commit(local.id())?.tree()?;
	let remote_tree = repo.find_commit(remote.id())?.tree()?;
	let ancestor = repo
		.find_commit(repo.merge_base(local.id(), remote.id())?)?
		.tree()?;
	
	let mut idx = repo.merge_trees(&ancestor, &local_tree, &remote_tree, None)?;

	if idx.has_conflicts() {
		repo.checkout_index(Some(&mut idx), None)?;
		return Ok(GitResult::Conflicted {
			conflicted_files: idx.conflicts()?.count(),
		});
	}
	
	if idx.is_empty() {
		return Ok(GitResult::NothingToDo);
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
	Ok(GitResult::Clean)
}

pub fn is_conflicted(git_path: &str) -> Result<GitResult, DynError> {
	let repo = match Repository::open(git_path) {
		Ok(r) => r,
		Err(e) => {
			return Err(Box::new(CmError {
				severity: Severity::Warning,
				message: format!("couldn't find a git repo in '{}': {}", git_path, e),
			}))
		}
	};

	let idx = repo.index()?;

	if idx.has_conflicts() {
		Ok(GitResult::Conflicted {
			conflicted_files: idx.conflicts()?.count(),
		})
	} else {
		Ok(GitResult::Clean)
	}
}

pub fn pull(git_path: &str, caf_path: &str, tag: String) -> Result<GitResult, DynError> {
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
		Ok(GitResult::NothingToDo)
	}
}

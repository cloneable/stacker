# Stacker

Easy git rebasing of stacked feature branches

## Audience

Stacker is recommended for a workflow where
*  individual contributors work in the same repository,
*  commits are made to dev branches forked from the main or a topic branch,
*  each dev branch is used by the one contributor and
*  dev branches are merged after code review into the base branch.

Stacker helps with managing entire *stacks* of these dev branches, i.e. when
they are branched off of and depend on each other, allowing for a more rapid
development. While one branch is undergoing review another branch can be stacked
onto it and worked in.

## Installation

Requires Go toolchain 1.17 or later.

```sh
go install github.com/cloneable/stacker@latest
```

Make sure `stacker` is in your $PATH.

```sh
export PATH=$(go env GOPATH)/bin:$PATH
```

## Usage

* `stacker init` checks and creates any stacker-related refs.
* `stacker clean` removes any stacker-related refs and settings.
* `stacker start <branch>` starts new branch, marks it for remote tracking and
  switches to it.
* `stacker push` pushes the current branch to remote and marks them for
  tracking.
* `stacker rebase` rebases the current branch.
* `stacker sync` shortcut for `git fetch --all --prune`.

## Under the Hood

### Tracking Refs

Stacker uses custom refs to track branches:

*  `refs/stacker/base/<branchname>`

   A symref to the parent branch in the stack. Created when a branch is created.
   Only updated when a branch is moved within the stack. Deleted when either
   branch is deleted.

*  `refs/stacker/start/<branchname>`

   The commit where the branch starts. Created when a branch is created. Updated
   after a rebase. Deleted when the branch is deleted.

*  `refs/stacker/remote/<branchname>`

   The commit where the remote branch head is expected to be. Created when a
   branch is pushed for the first time. Updated after a push. Deleted when the
   branch is deleted.

Stacker does not update/delete any refs outside `refs/stacker/`.

### Config

Stacker puts a few settings into `$REPO/.git/config`:

*  `log.excludeDecoration = refs/stacker/`

   To hide the trackings refs in `git log` output.

*  `transfer.hideRefs = refs/stacker/`

   Probably over-paranoid. Just so there's no possibility these refs leave the
   local repo.

*  `stacker.*`

   Any stacker specific settings.

Stacker will only touch the repo's config. It's safe to make these settings in
`--global` or `--system`.

### Git Commands

Stacker mainly uses two commands to manage branches:

*  `git rebase --committer-date-is-author-date --onto refs/stacker/base/<branch> refs/stacker/start/<branch> <branch>`

   Rebases a branch onto its base branch starting at start marker. If there are
   conflict, rebasing will stop. Fix conflicts and use `--continue` or
   `--abort`. After a successful rebase the start marker ref will be moved.

*  `git push --set-upstream --force-with-lease=<branch>:<expected-commit> <remote> <branch>:<branch>`

   Pushes a branch to remote, potentially replacing the commit chain, so the
   remote branch looks like the local branch.

In addition, Stacker uses a few more commands to help keeping track of things,
like `for-each-ref`, `update-ref`, `symbolic-ref`, `check-ref-format`.

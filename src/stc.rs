use crate::git::*;
use ::std::clone::Clone;
use ::std::option::Option::{self, Some};
use ::std::result::Result::{self, Err, Ok};
use ::std::string::String;
use ::std::string::ToString;
use ::std::vec::Vec;

pub struct STC<G: Git> {
    git: G,
}

impl<G: Git> STC<G> {
    pub fn new(git: G) -> Self {
        STC { git }
    }

    pub fn init(&self) -> Result<(), Status> {
        let g = &self.git;
        g.config_add("transfer.hideRefs", STACKER_REF_PREFIX)?;
        g.config_add("log.excludeDecoration", STACKER_REF_PREFIX)?;

        // TODO: read refs, branches, remotes
        // TODO: validate stacker refs against branches
        // TODO: determine list of needed refs
        // TODO: print and create list of created refs

        Ok(())
    }

    pub fn clean(&self) -> Result<(), Status> {
        let g = &self.git;
        g.config_unset_pattern("transfer.hideRefs", STACKER_REF_PREFIX)?;
        g.config_unset_pattern("log.excludeDecoration", STACKER_REF_PREFIX)?;

        // TODO: for each branch
        // TODO: ... check if fully merged
        // TODO: ... check if remote ref == local branch
        // TODO: ... delete stacker refs
        // TODO: ... or print warning

        Ok(())
    }

    pub fn start(&self, name: String) -> Result<(), Status> {
        let g = &self.git;
        let refs = g.snapshot()?;
        let base_branch = g.head()?;
        let new_name = g.check_branchname(&name)?;
        g.create_branch(&new_name, &base_branch)?;
        g.switch_branch(&new_name)?;
        g.create_symref(
            &new_name.stacker_base_refname(),
            &base_branch.refname(),
            "stacker: base branch marker",
        )?;
        let base_ref = refs.get(&base_branch.refname()).ok_or(Status::with(1))?;
        g.create_ref(&new_name.stacker_start_refname(), &base_ref.objectname)?;

        Ok(())
    }

    pub fn push(&self) -> Result<(), Status> {
        let g = &self.git;

        let expected_commit: ObjectName;
        {
            let refs = g.snapshot()?;
            let cur_branch = g.head()?;
            let base_symref = refs
                .get(&cur_branch.stacker_base_refname())
                .ok_or(Status::with(1))?;
            let base_ref = refs
                .get(&base_symref.symref_target)
                .ok_or(Status::with(1))?;
            if let Some(remote_ref) = refs.get(&cur_branch.stacker_remote_refname()) {
                expected_commit = remote_ref.objectname.clone();
            } else {
                expected_commit = ObjectName::new(NON_EXISTANT_OBJECT.to_string());
            }
            g.push(&cur_branch, &base_ref.remote, &expected_commit)?;
        }
        {
            let refs = g.snapshot()?;
            let cur_branch = g.head()?;
            let cur_ref = refs.get(&cur_branch.refname()).ok_or(Status::with(1))?;
            g.update_ref(
                &cur_branch.stacker_remote_refname(),
                &cur_ref.objectname,
                &expected_commit,
            )?;
        }

        Ok(())
    }

    pub fn rebase(&self) -> Result<(), Status> {
        let g = &self.git;

        let refs = g.snapshot()?;
        let branch = g.head()?;
        let base_ref = refs
            .get(&branch.stacker_base_refname())
            .ok_or(Status::with(1))?;
        let start_ref = refs
            .get(&branch.stacker_start_refname())
            .ok_or(Status::with(1))?;
        g.rebase_onto(&branch)?;
        g.update_ref(
            &branch.stacker_start_refname(),
            &base_ref.objectname,
            &start_ref.objectname,
        )?;

        Ok(())
    }

    pub fn sync(&self) -> Result<(), Status> {
        let g = &self.git;

        g.fetch_all_prune()?;

        Ok(())
    }

    pub fn fix(&self, branch: Option<String>, base: Option<String>) -> Result<(), Status> {
        let g = &self.git;

        let refs = g.snapshot()?;
        // TODO: this is hacky. refactor.
        if let Some(branchname) = branch {
            if let Some(base_branchname) = base {
                let branch = g.check_branchname(&branchname)?;
                let base_branch = g.check_branchname(&base_branchname)?;
                if let Some(base_symref) = refs.get(&branch.stacker_base_refname()) {
                    if base_symref.symref_target != base_branch.refname() {
                        return Err(Status::new(
                            1,
                            Vec::<u8>::new(),
                            "base branch already defined".as_bytes().to_vec(),
                        ));
                    }
                } else {
                    g.create_symref(
                        &branch.stacker_base_refname(),
                        &base_branch.refname(),
                        "stacker: set base branch",
                    )?;
                }
                if let Some(_start_ref) = refs.get(&branch.stacker_start_refname()) {
                    // TODO: check if base or ancestor of base
                } else {
                    let forkpoint = g.forkpoint(&base_branch.refname(), &branch.refname())?;
                    g.create_ref(&branch.stacker_start_refname(), &forkpoint)?;
                }
            } else {
                return Err(Status::new(
                    1,
                    Vec::<u8>::new(),
                    "base not specified".as_bytes().to_vec(),
                ));
            }
        }

        let refs = g.snapshot()?;
        for branch in g.tracked_branches()? {
            if refs.get(&branch.refname()).is_none() {
                if let Some(r) = refs.get(&branch.stacker_base_refname()) {
                    g.delete_symref(&r.name)?;
                }
                if let Some(r) = refs.get(&branch.stacker_start_refname()) {
                    g.delete_ref(&r.name, &r.objectname)?;
                }
                if let Some(r) = refs.get(&branch.stacker_remote_refname()) {
                    g.delete_ref(&r.name, &r.objectname)?;
                }
            }
        }

        let refs = g.snapshot()?;
        for branch in g.tracked_branches()? {
            // for each existing branch that's somehow still being tracked:
            let base_symref_name = branch.stacker_base_refname();
            let start_refname = branch.stacker_start_refname();
            if let Some(base_symref) = refs.get(&base_symref_name) {
                // there's a base symref
                if refs.get(&start_refname).is_none() {
                    // but no start ref
                    if refs.get(&base_symref.symref_target).is_none() {
                        // TODO: base branch doesn't exist (anymore)
                        continue;
                    }
                    // figure out forkpoint from what the base symref points to and the branch
                    // TODO: forkpoint can fail
                    let forkpoint = g.forkpoint(&base_symref.symref_target, &branch.refname())?;
                    // write the commit as start ref
                    g.create_ref(&branch.stacker_start_refname(), &forkpoint)?;
                }
            } else {
                // there's no base symref
                if let Some(_start_ref) = refs.get(&start_refname) {
                    // but there's a start ref
                    // TODO: check for branch at that commit? consult reflog?
                }
            }
        }

        // TODO: no /base/, but /start/ -> look for branch head at /start/, set /base/
        // TODO: no /start/, but /base/ -> use git merge-base to find fork point
        // TODO: no /start/ nor /base/ -> do nothing, offer explicit way to track
        // TODO: no /remote/, but remote branch exists? -> set ref, if ancestor, if not -> error
        // TODO: no remote branch, but /remote/ -> delete ref (check origin?)

        Ok(())
    }
}

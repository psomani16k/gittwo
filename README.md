# gittwo 

*A wip wrapper around [git2](https://crates.io/crates/git2) with a git cli like api.*

## What works?

*Level 1 items are the commands without any flags, level 2 items are the list of flags. Anything not in this list isn't supported by default. Whatever is in the list but not checked will be added eventually (barring lack of support on the libgit2 side).*

### Commands
- [x] Clone
    - [x] `--single-branch`
    - [x] `--branch`
    - [x] `--bare`
    - [x] `--depth`
    - [x] `--recusive`
- [x] Init
    - [x] `--bare`
    - [x] `--initial-branch`
    - [x] `--separate-git-dir`
- [x] Add
    - [x] `--update`
    - [x] `--dry-run`
- [x] Commit 
    - [x] `--message`
    - [x] `--allow-empty-message`
- [x] Push 
    - [x] `--set-upstream`
    - [x] `--all`
- [ ] Pull
    - [ ] `--unshallow`
    - [ ] `--rebase`
- [ ] Remote 
    - [ ] `add`
    - [ ] `remove`
    - [ ] `set-head`
    - [ ] `set-branch`
- [ ] Restore
    - [ ] `--staged`
- [ ] Checkout 
- [ ] Status 
- [ ] Branch
- [ ] Stash
- [ ] Fetch
- [ ] Merge
- [ ] Reset
- [ ] Submodule

### Credentials
- [x] HTTPS
- [ ] SSH

## License

This project is licensed under either of

- Apache License, Version 2.0, (LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0)
- MIT license (LICENSE-MIT or https://opensource.org/licenses/MIT)

at your option.

### Contributions
Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in gittwo by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

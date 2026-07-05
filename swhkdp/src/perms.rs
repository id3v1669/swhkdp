use std::path::Path;

pub fn root_write_only(uid: u32, mode: u32) -> bool {
    uid == 0 && mode & 0o022 == 0
}

pub fn chain_is_root_write_only(path: &Path) -> bool {
    chain_ok(
        path.ancestors()
            .skip(1)
            .filter(|a| !a.as_os_str().is_empty())
            .map(|ancestor| nix::sys::stat::stat(ancestor).ok().map(|st| (st.st_uid, st.st_mode))),
    )
}

fn chain_ok(entries: impl Iterator<Item = Option<(u32, u32)>>) -> bool {
    for entry in entries {
        match entry {
            Some((uid, mode)) if root_write_only(uid, mode) => {}
            _ => return false,
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_root_file_0644() {
        assert!(root_write_only(0, 0o100644));
    }

    #[test]
    fn accepts_root_file_0600() {
        assert!(root_write_only(0, 0o100600));
    }

    #[test]
    fn accepts_root_dir_0755() {
        assert!(root_write_only(0, 0o040755));
    }

    #[test]
    fn rejects_non_root_owner() {
        assert!(!root_write_only(1000, 0o100644));
    }

    #[test]
    fn rejects_group_writable() {
        assert!(!root_write_only(0, 0o100664));
    }

    #[test]
    fn rejects_other_writable() {
        assert!(!root_write_only(0, 0o100646));
    }

    #[test]
    fn chain_accepts_all_root_write_only_ancestors() {
        let entries = [Some((0u32, 0o040755u32)), Some((0, 0o040711)), Some((0, 0o040700))];
        assert!(chain_ok(entries.into_iter()));
    }

    #[test]
    fn chain_rejects_non_root_owned_ancestor() {
        let entries = [Some((0u32, 0o040755u32)), Some((1000, 0o040755)), Some((0, 0o040700))];
        assert!(!chain_ok(entries.into_iter()));
    }

    #[test]
    fn chain_rejects_group_writable_ancestor() {
        let entries = [Some((0u32, 0o040755u32)), Some((0, 0o040775)), Some((0, 0o040700))];
        assert!(!chain_ok(entries.into_iter()));
    }

    #[test]
    fn chain_rejects_when_any_ancestor_stat_failed() {
        let entries = [Some((0u32, 0o040755u32)), None];
        assert!(!chain_ok(entries.into_iter()));
    }
}

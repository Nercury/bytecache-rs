use std::path::{ PathBuf };
use std::convert::AsRef;

/// Replaces invalid characters in potential file name with characters that are valid on this OS.
///
/// Warning, so far linux-only.
pub fn replace_invalid_path_chars(key: &str) -> String {
    let mut res = String::with_capacity(key.len());

    for c in key.chars() {
        res.push(match c {
            '/' => '_',
            c => c,
        });
    }

    res
}

/// Construct a valid path for provided string key.
///
/// `subdirs`: Maximum number of subdirectories to generate for this key.
/// `subdir_len`: Subdir name length.
pub fn construct(key: &str, subdirs: usize, subdir_len: usize) -> Option<PathBuf> {
    if let "" = key {
        return None;
    }

    let key = &replace_invalid_path_chars(key);

    let mut path = PathBuf::new();
    let mut dir_offset = 0;

    for _ in 0..subdirs {
        let next_offset = dir_offset + subdir_len;
        if next_offset > key.len() {
            break;
        }
        path.push(&key[dir_offset..next_offset]);
        dir_offset = next_offset;
    }

    path.push(key);

    Some(path)
}

/// Construct a valid path for provided string key and default subdir parameters.
///
/// Uses `DEF_SUBDIRS` and `DEF_SUBDIR_LEN`.
pub fn construct_def(key: &str) -> Option<PathBuf> {
    construct(key, DEF_SUBDIRS, DEF_SUBDIR_LEN)
}

/// Helper to construct paths to binary blob and its meta data.
pub struct PathGen {
    base: Option<PathBuf>,
}

impl PathGen {
    pub fn new(key: &str, subdirs: usize, subdir_len: usize) -> PathGen {
        PathGen {
            base: construct(key, subdirs, subdir_len),
        }
    }

    pub fn default(key: &str) -> PathGen {
        PathGen {
            base: construct_def(key),
        }
    }

    /// Get path to binary blob.
    pub fn file_path(&self) -> Option<PathBuf> {
        self.base.clone()
    }

    /// Get path to blob meta data.
    pub fn meta_path(&self) -> Option<PathBuf> {
        if let Some(ref buf) = self.base {
            let mut meta_buf = buf.clone();

            let file_name = match meta_buf.file_name() {
                Some(n) => Some(n.to_string_lossy().into_owned()),
                None => None,
            };

            match file_name {
                Some(name) => {
                    let name: String = [name.as_ref(), "meta"].connect(".");
                    meta_buf.set_file_name(name);
                },
                None => return None,
            };

            return Some(meta_buf);
        }

        None
    }
}

/// Default number of subdirs to generate.
///
/// For example, if the text is "FSHFJKDS", generate FS/HF/JK.
pub const DEF_SUBDIRS: usize = 3;

/// Default subdir len.
///
/// 2 char subdir looks like FS/HF/JK.
/// 3 char subdir looks like FSH/FJK.
pub const DEF_SUBDIR_LEN: usize = 2;

#[cfg(test)]
mod test {
    use std::path::PathBuf;
    use super::*;

    #[test]
    fn empty_key_has_no_path() {
        assert_eq!(construct_def(""), None);
    }

    #[test]
    fn no_subdirs_are_generated_if_short_path() {
        assert_eq!(construct_def("a"), Some(PathBuf::from("a")));
    }

    #[test]
    fn single_subdir_is_generated() {
        assert_eq!(construct_def("aa"), Some(PathBuf::from("aa/aa")));
    }

    #[test]
    fn single_subdir_is_generated_if_short() {
        assert_eq!(construct_def("aab"), Some(PathBuf::from("aa/aab")));
    }

    #[test]
    fn two_subdirs_are_generated() {
        assert_eq!(construct_def("aabb"), Some(PathBuf::from("aa/bb/aabb")));
    }

    #[test]
    fn two_subdirs_are_generated_if_short() {
        assert_eq!(construct_def("aabbc"), Some(PathBuf::from("aa/bb/aabbc")));
    }

    #[test]
    fn three_subdirs_are_generated() {
        assert_eq!(construct_def("aabbcc"), Some(PathBuf::from("aa/bb/cc/aabbcc")));
    }

    #[test]
    fn three_subdirs_are_generated_if_short() {
        assert_eq!(construct_def("aabbccd"), Some(PathBuf::from("aa/bb/cc/aabbccd")));
    }

    #[test]
    fn only_three_subdirs_should_be_generated() {
        assert_eq!(construct_def("aabbccdd"), Some(PathBuf::from("aa/bb/cc/aabbccdd")));
        assert_eq!(construct_def("aabbccddee"), Some(PathBuf::from("aa/bb/cc/aabbccddee")));
    }

    #[test]
    fn different_subdir_len_works() {
        assert_eq!(construct("aabbccdd", 1, 4), Some(PathBuf::from("aabb/aabbccdd")));
        assert_eq!(construct("aabbccdd", 4, 1), Some(PathBuf::from("a/a/b/b/aabbccdd")));
        assert_eq!(construct("aabbccdd", 0, 0), Some(PathBuf::from("aabbccdd")));
        assert_eq!(construct("a", 0, 0), Some(PathBuf::from("a")));
        assert_eq!(construct("aabbccdd", 0, 1), Some(PathBuf::from("aabbccdd")));
        assert_eq!(construct("aabbccdd", 1, 0), Some(PathBuf::from("aabbccdd")));
        assert_eq!(construct("aabbccdd", 2, 3), Some(PathBuf::from("aab/bcc/aabbccdd")));
    }

    #[test]
    fn should_replace_invalid_path_chars() {
        assert_eq!("valid", &replace_invalid_path_chars("valid"));
        assert_eq!("invalid_file_name", &replace_invalid_path_chars("invalid/file/name"));
    }

    #[test]
    fn invalid_path_chars_should_be_replaced() {
        assert_eq!(construct_def("aab/ccdd"), Some(PathBuf::from("aa/b_/cc/aab_ccdd")));
    }

    #[test]
    fn path_gen_should_have_correct_file_path() {
        assert_eq!(PathGen::default("aab").file_path(), Some(PathBuf::from("aa/aab")));
        assert_eq!(PathGen::default("aabbcc").file_path(), Some(PathBuf::from("aa/bb/cc/aabbcc")));
    }

    #[test]
    fn path_gen_should_have_correct_meta_path() {
        assert_eq!(PathGen::default("aab").meta_path(), Some(PathBuf::from("aa/aab.meta")));
        assert_eq!(PathGen::default("aabbcc").meta_path(), Some(PathBuf::from("aa/bb/cc/aabbcc.meta")));
    }
}

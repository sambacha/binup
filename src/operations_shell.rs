use crate::constants::{
    APP_NAME, SHELL_SCRIPT_END_MARKER as E_MARKER, SHELL_SCRIPT_HEADER as HEADER,
    SHELL_SCRIPT_START_MARKER as S_MARKER,
};
use anyhow::{anyhow, bail, Context, Result};
use bstr::ByteSlice;
use bstr::ByteVec;
use indoc::formatdoc;
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};

fn get_shell_script_gup_content(bin_path: &Path, path: &Path) -> Result<Vec<u8>> {
    let mut result: Vec<u8> = Vec::new();

    let bin_path_str = match bin_path.to_str() {
        Some(s) => s,
        None =>  bail!("Could not create UTF-8 string from passed-in binary application path. Currently only valid UTF-8 paths are supported"),
    };

    result.extend_from_slice(S_MARKER);
    result.extend_from_slice(HEADER);
    if path.file_name().unwrap() == ".zshrc" {
        append_zsh_content(&mut result, bin_path_str);
    } else {
        append_sh_content(&mut result, bin_path_str);
    }
    result.extend_from_slice(b"\n");
    result.extend_from_slice(E_MARKER);

    Ok(result)
}

fn append_zsh_content(buf: &mut Vec<u8>, path_str: &str) {
    let content = formatdoc!(
        "
            path=('{}' $path)
            export PATH
        ",
        path_str
    );

    buf.extend_from_slice(content.as_bytes());
}

fn append_sh_content(buf: &mut Vec<u8>, path_str: &str) {
    let content = formatdoc!(
        "
            case \":$PATH:\" in
                *:{0}:*)
                    ;;

                *)
                    export PATH={0}${{PATH:+:${{PATH}}}}
                    ;;
            esac
        ",
        path_str
    );
    buf.extend_from_slice(content.as_bytes());
}

fn match_markers(buffer: &[u8]) -> Result<Option<(usize, usize)>> {
    let start_marker = buffer.find(S_MARKER);
    let end_marker = buffer.find(E_MARKER);

    let (start_marker, end_marker) = match (start_marker, end_marker) {
        (Some(sidx), Some(eidx)) => {
            let second_s_marker = buffer[sidx + S_MARKER.len()..].find(S_MARKER);
            let second_e_marker = buffer[eidx + E_MARKER.len()..].find(E_MARKER);

            if second_s_marker.is_some() || second_e_marker.is_some() {
                bail!("Found multiple startup script sections from {}.", APP_NAME);
            }
            (sidx, eidx)
        }
        (None, None) => {
            return Ok(None);
        }
        (_, None) => {
            bail!(
                "Found an opening marker but no end marker of {} section.",
                APP_NAME
            );
        }
        (None, _) => {
            bail!(
                "Found an end marker but no opening marker of {} section.",
                APP_NAME
            );
        }
    };

    Ok(Some((start_marker, end_marker + E_MARKER.len())))
}

fn add_path_to_specific_file(bin_path: &Path, path: &Path) -> Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(path)
        .with_context(|| format!("Failed to open file {}.", path.display()))?;

    let mut buffer: Vec<u8> = Vec::new();

    file.read_to_end(&mut buffer)
        .with_context(|| format!("Failed to read data from file {}.", path.display()))?;

    let existing_code_pos = match_markers(&buffer).with_context(|| {
        format!(
            "Error occured while searching gup shell startup script section in {}",
            path.display()
        )
    })?;

    let new_content = get_shell_script_gup_content(bin_path, &path).with_context(|| {
        format!(
            "Error occured while generating gup shell startup script section for {}",
            path.display()
        )
    })?;

    match existing_code_pos {
        Some(pos) => {
            buffer.replace_range(pos.0..pos.1, &new_content);
        }
        None => {
            buffer.extend_from_slice(b"\n");
            buffer.extend_from_slice(&new_content);
            buffer.extend_from_slice(b"\n");
        }
    };

    file.rewind().unwrap();
    file.set_len(0).unwrap();
    file.write_all(&buffer).unwrap();
    file.sync_all().unwrap();

    Ok(())
}

fn remove_path_from_specific_file(path: &Path) -> Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .with_context(|| format!("Failed to open file: {}", path.display()))?;

    let mut buffer: Vec<u8> = Vec::new();

    file.read_to_end(&mut buffer)?;

    let existing_code_pos = match_markers(&buffer).with_context(|| {
        format!(
            "Error occured while searching gup shell startup script section in {}",
            path.display()
        )
    })?;

    if let Some(pos) = existing_code_pos {
        buffer.replace_range(pos.0..pos.1, "");

        file.rewind().unwrap();
        file.set_len(0).unwrap();
        file.write_all(&buffer).unwrap();
        file.sync_all().unwrap();
    }

    Ok(())
}

pub fn find_shell_scripts_to_be_modified(add_case: bool) -> Result<Vec<PathBuf>> {
    let home_dir = std::env::var("HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| user_dirs::home_dir().ok())
        .ok_or_else(|| anyhow!("Failed to get home directory."))?;

    let paths_to_test: Vec<PathBuf> = vec![
        home_dir.join(".bashrc"),
        home_dir.join(".profile"),
        home_dir.join(".bash_profile"),
        home_dir.join(".bash_login"),
        home_dir.join(".zshrc"),
    ];

    let result = paths_to_test
        .iter()
        .filter(|p| {
            p.exists()
                || (add_case
                    && p.file_name().map(|name| name == ".zshrc").unwrap_or(false)
                    && std::env::consts::OS == "macos")
        })
        .cloned()
        .collect();
    Ok(result)
}

pub fn add_binfolder_to_path_in_shell_scripts(bin_path: &Path) -> Result<()> {
    let paths = find_shell_scripts_to_be_modified(true)?;

    for p in paths {
        add_path_to_specific_file(bin_path, &p)?;
    }
    Ok(())
}

pub fn remove_binfolder_from_path_in_shell_scripts() -> Result<()> {
    let paths = find_shell_scripts_to_be_modified(false)?;

    for p in paths {
        remove_path_from_specific_file(&p)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests would ideally use a temporary directory and mock files
    // to avoid modifying user's actual shell scripts and to ensure reproducibility.
    // For now, they test the marker logic.

    #[test]
    fn match_markers_none_without_markers() {
        let inp: &[u8] = b"Some input\n";
        let res = match_markers(inp);
        assert!(res.is_ok());
        let res = res.unwrap();
        assert!(res.is_none());
    }

    #[test]
    fn match_markers_returns_correct_indices() {
        let mut inp: Vec<u8> = Vec::new();
        let start_bytes = b"Some random bytes.";
        let middle_bytes = b"More bytes.";
        let end_bytes = b"Final bytes.";
        inp.extend_from_slice(start_bytes);
        inp.extend_from_slice(S_MARKER);
        inp.extend_from_slice(middle_bytes);
        inp.extend_from_slice(E_MARKER);
        inp.extend_from_slice(end_bytes);

        let res = match_markers(&inp);
        assert!(res.is_ok());
        let res = res.unwrap();
        assert!(res.is_some());
        let (sidx, eidx) = res.unwrap();

        assert_eq!(sidx, start_bytes.len());
        let expected_eidx =
            start_bytes.len() + S_MARKER.len() + middle_bytes.len() + E_MARKER.len();
        assert_eq!(eidx, expected_eidx);
    }

    #[test]
    fn match_markers_returns_err_without_start() {
        let mut inp: Vec<u8> = Vec::new();
        let start_bytes = b"Some random bytes.";
        let middle_bytes = b"More bytes.";
        let end_bytes = b"Final bytes.";
        inp.extend_from_slice(start_bytes);
        inp.extend_from_slice(middle_bytes);
        inp.extend_from_slice(E_MARKER);
        inp.extend_from_slice(end_bytes);

        let res = match_markers(&inp);
        assert!(res.is_err());
    }

    #[test]
    fn match_markers_returns_err_without_end() {
        let mut inp: Vec<u8> = Vec::new();
        let start_bytes = b"Some random bytes.";
        let middle_bytes = b"More bytes.";
        let end_bytes = b"Final bytes.";
        inp.extend_from_slice(start_bytes);
        inp.extend_from_slice(S_MARKER);
        inp.extend_from_slice(middle_bytes);
        inp.extend_from_slice(end_bytes);

        let res = match_markers(&inp);
        assert!(res.is_err());
    }

    #[test]
    fn match_markers_returns_err_with_multiple_start() {
        let mut inp: Vec<u8> = Vec::new();
        let start_bytes = b"Some random bytes.";
        let middle_bytes = b"More bytes.";
        let end_bytes = b"Final bytes.";
        inp.extend_from_slice(S_MARKER);
        inp.extend_from_slice(start_bytes);
        inp.extend_from_slice(S_MARKER);
        inp.extend_from_slice(middle_bytes);
        inp.extend_from_slice(E_MARKER);
        inp.extend_from_slice(end_bytes);

        let res = match_markers(&inp);
        assert!(res.is_err());
    }

    #[test]
    fn match_markers_returns_err_with_multiple_end() {
        let mut inp: Vec<u8> = Vec::new();
        let start_bytes = b"Some random bytes.";
        let middle_bytes = b"More bytes.";
        let end_bytes = b"Final bytes.";
        inp.extend_from_slice(start_bytes);
        inp.extend_from_slice(S_MARKER);
        inp.extend_from_slice(middle_bytes);
        inp.extend_from_slice(E_MARKER);
        inp.extend_from_slice(end_bytes);
        inp.extend_from_slice(E_MARKER);

        let res = match_markers(&inp);
        assert!(res.is_err());
    }
}

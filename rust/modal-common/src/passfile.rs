use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};

use crate::keypair::Keypair;
// use rpassword::read_password;

pub const MOD_PASSFILE_EXT: &str = "mod_passfile";
pub const LEGACY_MODAL_PASSFILE_EXT: &str = "modal_passfile";
pub const PUBLIC_ID_EXT: &str = "id";

#[derive(Clone)]
pub struct Passfile {
    pub filepath: PathBuf,
    pub keypair: Keypair,
}

impl Passfile {
    pub async fn load_file(filepath: PathBuf, _interactive: bool) -> Result<Self> {
        // TODO if interactive ask for password if encrypted
        let keypair = Keypair::from_json_file(filepath.to_str().unwrap())?;
        Ok(Self { filepath, keypair })
    }
}

/// Validate a user-supplied identity name. Names may contain `/` namespaces
/// (`example/alice`) but cannot escape the passfile or id directories.
pub fn validate_identity_name(name: &str) -> Result<()> {
    if name.is_empty() {
        anyhow::bail!("Identity name cannot be empty");
    }
    if name.starts_with('/') || name.starts_with('\\') || Path::new(name).is_absolute() {
        anyhow::bail!("Identity name cannot be an absolute path");
    }
    if name.contains('\\') {
        anyhow::bail!("Identity name cannot contain backslashes");
    }
    for part in name.split('/') {
        if part.is_empty() || part == "." || part == ".." {
            anyhow::bail!(
                "Identity name '{}' is invalid: empty, '.', and '..' segments are not allowed",
                name
            );
        }
    }
    Ok(())
}

/// `$MODALITY_HOME/.modality/passfiles` when set, otherwise `~/.modality/passfiles`.
pub fn default_named_passfile_dir() -> Result<PathBuf> {
    Ok(modality_dir()?.join("passfiles"))
}

/// `$MODALITY_HOME/.modality/ids` when set, otherwise `~/.modality/ids`.
pub fn default_named_id_dir() -> Result<PathBuf> {
    Ok(modality_dir()?.join("ids"))
}

/// Build `<dir>/<name>.mod_passfile`, creating parent directories.
/// When `dir` is `None`, uses [`default_named_passfile_dir`].
pub fn named_passfile_create_path(name: &str, dir: Option<&Path>) -> Result<PathBuf> {
    validate_identity_name(name)?;
    let base = match dir {
        Some(d) => d.to_path_buf(),
        None => {
            let default_dir = default_named_passfile_dir()?;
            std::fs::create_dir_all(&default_dir)?;
            default_dir
        }
    };
    let filepath = named_passfile_path_in(&base, name)?;
    if let Some(parent) = filepath.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(filepath)
}

/// `base_dir/<name>.mod_passfile` without creating directories.
pub fn named_passfile_path_in(base_dir: &Path, name: &str) -> Result<PathBuf> {
    validate_identity_name(name)?;
    Ok(base_dir.join(format!("{}.{}", name, MOD_PASSFILE_EXT)))
}

/// `base_dir/<name>.id` without creating directories.
pub fn named_id_path_in(base_dir: &Path, name: &str) -> Result<PathBuf> {
    validate_identity_name(name)?;
    Ok(base_dir.join(format!("{}.{}", name, PUBLIC_ID_EXT)))
}

/// Write a public ID to `~/.modality/ids/<name>.id`.
pub fn write_named_public_id(name: &str, public_id: &str) -> Result<PathBuf> {
    let dir = default_named_id_dir()?;
    std::fs::create_dir_all(&dir)?;
    write_named_public_id_in(&dir, name, public_id)
}

/// Write a public ID under an explicit ids directory (for tests).
pub fn write_named_public_id_in(ids_dir: &Path, name: &str, public_id: &str) -> Result<PathBuf> {
    let filepath = named_id_path_in(ids_dir, name)?;
    if let Some(parent) = filepath.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&filepath, public_id)?;
    Ok(filepath)
}

/// Resolve a passfile path or identity name to an existing private passfile.
pub fn resolve_passfile_path(name_or_path: &str) -> Result<PathBuf> {
    let home = modality_home()?;
    let cwd = std::env::current_dir()?;
    resolve_passfile_path_in(name_or_path, &home, &cwd)
}

/// Resolve a passfile using explicit home and cwd roots (for tests).
pub fn resolve_passfile_path_in(name_or_path: &str, home: &Path, cwd: &Path) -> Result<PathBuf> {
    let mut looked = Vec::new();
    if let Some(path) = existing_direct_file(name_or_path, cwd, &mut looked) {
        return Ok(path);
    }

    if validate_identity_name(name_or_path).is_ok() {
        for candidate in named_passfile_lookup_candidates(home, cwd, name_or_path) {
            looked.push(candidate.display().to_string());
            if candidate.is_file() {
                warn_if_named_identity_conflict(name_or_path, home, cwd, &candidate);
                return Ok(candidate);
            }
        }
    }

    not_found(name_or_path, &looked)
}

/// Resolve a public ID from a path, named `.id` file, or named passfile.
pub fn resolve_public_id(name_or_path: &str) -> Result<String> {
    let home = modality_home()?;
    let cwd = std::env::current_dir()?;
    resolve_public_id_in(name_or_path, &home, &cwd)
}

/// Resolve a public ID using explicit home and cwd roots (for tests).
pub fn resolve_public_id_in(name_or_path: &str, home: &Path, cwd: &Path) -> Result<String> {
    let mut looked = Vec::new();
    if let Some(path) = existing_direct_file(name_or_path, cwd, &mut looked) {
        return public_id_from_file(&path);
    }

    if validate_identity_name(name_or_path).is_ok() {
        for candidate in named_id_lookup_candidates(home, cwd, name_or_path) {
            looked.push(candidate.display().to_string());
            if candidate.is_file() {
                warn_if_named_identity_conflict(name_or_path, home, cwd, &candidate);
                return public_id_from_file(&candidate);
            }
        }
        if let Ok(passfile) = resolve_passfile_path_in(name_or_path, home, cwd) {
            return public_id_from_file(&passfile);
        }
    }

    not_found(name_or_path, &looked)
}

/// Read a public address from a `.id` file or a passfile.
pub fn public_id_from_file(path: &Path) -> Result<String> {
    let path_str = path
        .to_str()
        .ok_or_else(|| anyhow!("Invalid file path: contains non-Unicode characters"))?;
    if let Ok(keypair) = Keypair::from_json_file(path_str) {
        return Ok(keypair.as_public_address());
    }
    let text = std::fs::read_to_string(path)?.trim().to_string();
    if text.is_empty() {
        anyhow::bail!("Public ID file is empty: {}", path.display());
    }
    Ok(text)
}

fn modality_home() -> Result<PathBuf> {
    if let Ok(dir) = std::env::var("MODALITY_HOME") {
        let path = PathBuf::from(dir);
        if !path.as_os_str().is_empty() {
            return Ok(path);
        }
    }
    dirs::home_dir().ok_or_else(|| anyhow!("Cannot find home directory"))
}

fn modality_dir() -> Result<PathBuf> {
    Ok(modality_home()?.join(".modality"))
}

fn existing_direct_file(
    name_or_path: &str,
    cwd: &Path,
    looked: &mut Vec<String>,
) -> Option<PathBuf> {
    let direct_path = PathBuf::from(name_or_path);
    let resolved_direct = if direct_path.is_absolute() {
        direct_path
    } else {
        cwd.join(&direct_path)
    };
    looked.push(format!("{} (direct path)", resolved_direct.display()));
    resolved_direct.is_file().then_some(resolved_direct)
}

fn named_passfile_lookup_candidates(home: &Path, cwd: &Path, name: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for ext in [MOD_PASSFILE_EXT, LEGACY_MODAL_PASSFILE_EXT] {
        paths.push(
            home.join(".modality")
                .join("passfiles")
                .join(format!("{}.{}", name, ext)),
        );
        paths.push(cwd.join(format!("{}.{}", name, ext)));
        paths.push(home.join(".modality").join(format!("{}.{}", name, ext)));
        paths.push(
            home.join(".modality")
                .join("identities")
                .join(format!("{}.{}", name, ext)),
        );
    }
    paths
}

/// Mismatch between a named public ID file and the corresponding passfile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedIdentityConflict {
    pub name: String,
    pub id_path: PathBuf,
    pub id_value: String,
    pub passfile_path: PathBuf,
    pub passfile_id: String,
}

impl NamedIdentityConflict {
    pub fn warning(&self, using: &Path) -> String {
        format!(
            "⚠️  Identity '{}' public ID file and passfile do not match.\n  ID file:  {} ({})\n  Passfile: {} ({})\n  Using:    {}",
            self.name,
            self.id_path.display(),
            self.id_value,
            self.passfile_path.display(),
            self.passfile_id,
            using.display()
        )
    }
}

/// If both a named `.id` and a passfile exist and their public IDs differ, describe the conflict.
pub fn named_identity_conflict(
    name: &str,
    home: &Path,
    cwd: &Path,
) -> Option<NamedIdentityConflict> {
    if validate_identity_name(name).is_err() {
        return None;
    }
    let id_path = named_id_lookup_candidates(home, cwd, name)
        .into_iter()
        .find(|path| path.is_file())?;
    let passfile_path = named_passfile_lookup_candidates(home, cwd, name)
        .into_iter()
        .find(|path| path.is_file())?;
    let id_value = public_id_from_file(&id_path).ok()?;
    let passfile_id = public_id_from_file(&passfile_path).ok()?;
    if id_value == passfile_id {
        return None;
    }
    Some(NamedIdentityConflict {
        name: name.to_string(),
        id_path,
        id_value,
        passfile_path,
        passfile_id,
    })
}

fn warn_if_named_identity_conflict(name: &str, home: &Path, cwd: &Path, using: &Path) {
    if let Some(conflict) = named_identity_conflict(name, home, cwd) {
        eprintln!("{}", conflict.warning(using));
    }
}

fn named_id_lookup_candidates(home: &Path, cwd: &Path, name: &str) -> Vec<PathBuf> {
    vec![
        home.join(".modality")
            .join("ids")
            .join(format!("{}.{}", name, PUBLIC_ID_EXT)),
        cwd.join(format!("{}.{}", name, PUBLIC_ID_EXT)),
    ]
}

fn not_found<T>(name_or_path: &str, looked: &[String]) -> Result<T> {
    anyhow::bail!(
        "Identity '{}' not found. Looked in:\n{}",
        name_or_path,
        looked
            .iter()
            .map(|p| format!("  - {}", p))
            .collect::<Vec<_>>()
            .join("\n")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn nested_name_builds_namespaced_path() {
        let base = PathBuf::from("/tmp/home/.modality/passfiles");
        let path = named_passfile_path_in(&base, "example/alice").unwrap();
        assert_eq!(path, base.join("example/alice.mod_passfile"));
        let id_path =
            named_id_path_in(Path::new("/tmp/home/.modality/ids"), "example/alice").unwrap();
        assert_eq!(
            id_path,
            PathBuf::from("/tmp/home/.modality/ids/example/alice.id")
        );
    }

    #[test]
    fn rejects_parent_directory_traversal() {
        let err = validate_identity_name("example/../alice").unwrap_err();
        assert!(err.to_string().contains(".."));
        assert!(validate_identity_name("../alice").is_err());
        assert!(validate_identity_name("alice/.").is_err());
        assert!(validate_identity_name("/alice").is_err());
        assert!(validate_identity_name("alice\\bob").is_err());
        assert!(validate_identity_name("").is_err());
        assert!(validate_identity_name("example//alice").is_err());
    }

    #[test]
    fn create_path_makes_namespace_parent_dirs() {
        let tmp = TempDir::new().unwrap();
        let path = named_passfile_create_path("example/alice", Some(tmp.path())).unwrap();
        assert_eq!(path, tmp.path().join("example/alice.mod_passfile"));
        assert!(path.parent().unwrap().is_dir());
    }

    #[test]
    fn explicit_existing_path_wins() {
        let home = TempDir::new().unwrap();
        let cwd = TempDir::new().unwrap();
        let local = cwd.path().join("alice.mod_passfile");
        fs::write(&local, "local").unwrap();

        let named_dir = home.path().join(".modality/passfiles");
        fs::create_dir_all(&named_dir).unwrap();
        fs::write(named_dir.join("alice.mod_passfile"), "named").unwrap();

        let resolved =
            resolve_passfile_path_in(local.to_str().unwrap(), home.path(), cwd.path()).unwrap();
        assert_eq!(resolved, local);
    }

    #[test]
    fn resolves_namespaced_name_under_passfiles() {
        let home = TempDir::new().unwrap();
        let cwd = TempDir::new().unwrap();
        let named = home
            .path()
            .join(".modality/passfiles/example/alice.mod_passfile");
        fs::create_dir_all(named.parent().unwrap()).unwrap();
        fs::write(&named, "named").unwrap();

        let resolved = resolve_passfile_path_in("example/alice", home.path(), cwd.path()).unwrap();
        assert_eq!(resolved, named);
    }

    #[test]
    fn missing_name_lists_lookup_locations() {
        let home = TempDir::new().unwrap();
        let cwd = TempDir::new().unwrap();
        let err = resolve_passfile_path_in("example/alice", home.path(), cwd.path()).unwrap_err();
        let message = err.to_string();
        assert!(message.contains("Identity 'example/alice' not found"));
        assert!(message.contains("direct path"));
        assert!(message.contains(".modality/passfiles/example/alice.mod_passfile"));
        assert!(message.contains("example/alice.mod_passfile"));
        assert!(message.contains(".modality/identities/example/alice.mod_passfile"));
    }

    #[test]
    fn falls_back_to_legacy_home_passfile_location() {
        let home = TempDir::new().unwrap();
        let cwd = TempDir::new().unwrap();
        let legacy = home.path().join(".modality/alice.modal_passfile");
        fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        fs::write(&legacy, "legacy").unwrap();

        let resolved = resolve_passfile_path_in("alice", home.path(), cwd.path()).unwrap();
        assert_eq!(resolved, legacy);
    }

    #[test]
    fn writes_and_resolves_public_id_without_passfile() {
        let home = TempDir::new().unwrap();
        let cwd = TempDir::new().unwrap();
        let ids_dir = home.path().join(".modality/ids");
        let written =
            write_named_public_id_in(&ids_dir, "example/alice", "12D3KooWexamplealice").unwrap();
        assert_eq!(written, ids_dir.join("example/alice.id"));
        assert_eq!(
            fs::read_to_string(&written).unwrap(),
            "12D3KooWexamplealice"
        );

        let resolved = resolve_public_id_in("example/alice", home.path(), cwd.path()).unwrap();
        assert_eq!(resolved, "12D3KooWexamplealice");
    }

    #[test]
    fn matching_named_id_and_passfile_are_not_a_conflict() {
        let home = TempDir::new().unwrap();
        let cwd = TempDir::new().unwrap();
        let keypair = Keypair::generate().unwrap();
        let public_id = keypair.as_public_address();

        let passfile = home
            .path()
            .join(".modality/passfiles/example/alice.mod_passfile");
        fs::create_dir_all(passfile.parent().unwrap()).unwrap();
        keypair.as_json_file(passfile.to_str().unwrap()).unwrap();
        write_named_public_id_in(
            &home.path().join(".modality/ids"),
            "example/alice",
            &public_id,
        )
        .unwrap();

        assert!(named_identity_conflict("example/alice", home.path(), cwd.path()).is_none());
    }

    #[test]
    fn mismatched_named_id_and_passfile_are_a_conflict() {
        let home = TempDir::new().unwrap();
        let cwd = TempDir::new().unwrap();
        let keypair = Keypair::generate().unwrap();
        let passfile_id = keypair.as_public_address();

        let passfile = home
            .path()
            .join(".modality/passfiles/example/alice.mod_passfile");
        fs::create_dir_all(passfile.parent().unwrap()).unwrap();
        keypair.as_json_file(passfile.to_str().unwrap()).unwrap();
        let id_path = write_named_public_id_in(
            &home.path().join(".modality/ids"),
            "example/alice",
            "12D3KooWmismatched",
        )
        .unwrap();

        let conflict = named_identity_conflict("example/alice", home.path(), cwd.path())
            .expect("mismatch should be reported");
        assert_eq!(conflict.id_path, id_path);
        assert_eq!(conflict.id_value, "12D3KooWmismatched");
        assert_eq!(conflict.passfile_path, passfile);
        assert_eq!(conflict.passfile_id, passfile_id);
        let warning = conflict.warning(&id_path);
        assert!(warning.contains("do not match"));
        assert!(warning.contains("12D3KooWmismatched"));
        assert!(warning.contains(&passfile_id));

        let resolved = resolve_public_id_in("example/alice", home.path(), cwd.path()).unwrap();
        assert_eq!(resolved, "12D3KooWmismatched");
        let resolved_passfile =
            resolve_passfile_path_in("example/alice", home.path(), cwd.path()).unwrap();
        assert_eq!(resolved_passfile, passfile);
    }
}

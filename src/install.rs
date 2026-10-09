use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const STEAM_APP_ID: &str = "3832490";
pub const DOCS_URL: &str = "https://github.com/avast-mod/docs";
const GAME_FOLDER: &str = "Antivirus Survivors 2003 Professional";
const GDPATCH_RELEASE: &str = "https://github.com/GDPatch/GDPatch/releases/latest/download";
const RUN_SCRIPT_URL: &str = "https://gdpatch.dev/run_with_gdpatch.sh";

const MODS: [(&str, &str); 2] = [
    (
        "avast_api",
        "https://github.com/avast-mod/api/releases/latest/download/avast_api.zip",
    ),
    (
        "avast_core",
        "https://github.com/avast-mod/core/releases/latest/download/avast_core.zip",
    ),
];

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Build {
    Windows,
    Proton,
    Linux,
    MacOS,
}

#[derive(Clone, Debug)]
pub struct Game {
    pub dir: PathBuf,
    pub build: Build,
}

impl Game {
    pub fn gdpatch_dir(&self) -> PathBuf {
        self.dir.join("GDPatch")
    }

    fn loader(&self) -> (PathBuf, &'static str) {
        match self.build {
            Build::Windows | Build::Proton => (self.dir.join("winmm.dll"), "gdpatch_loader.dll"),
            Build::Linux => (self.dir.join("libgdpatch_loader.so"), "libgdpatch_loader.so"),
            Build::MacOS => (self.dir.join("libgdpatch_loader.dylib"), "libgdpatch_loader.dylib"),
        }
    }

    pub fn gdpatch_installed(&self) -> bool {
        fs::read(self.loader().0).is_ok_and(|bytes| bytes.windows(7).any(|w| w == b"gdpatch"))
    }

    pub fn launch_options(&self) -> Option<String> {
        match self.build {
            Build::Windows => None,
            Build::Proton => Some(r#"WINEDLLOVERRIDES="winmm=n,b" %command%"#.into()),
            Build::Linux => Some("./run_with_gdpatch.sh %command%".into()),
            Build::MacOS => Some(format!(
                r#"DYLD_INSERT_LIBRARIES="{}" GDPATCH_ROOT_DIRECTORY="{}" %command%"#,
                self.loader().0.display(),
                self.gdpatch_dir().display()
            )),
        }
    }

    pub fn warnings(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.build == Build::MacOS {
            if cfg!(target_arch = "x86_64") {
                out.push("GDPatch only ships an Apple Silicon build. Intel Macs are not supported yet.".into());
            }
            if self.hardened_runtime() {
                out.push(
                    "This game build uses macOS Hardened Runtime, which ignores DYLD_INSERT_LIBRARIES. \
                     See the troubleshooting page in the docs."
                        .into(),
                );
            }
        }
        out
    }

    fn hardened_runtime(&self) -> bool {
        let Some(app) = entries(&self.dir).into_iter().find(|p| has_ext(p, "app")) else {
            return false;
        };
        let Ok(out) = Command::new("codesign")
            .args(["-dv", "--entitlements", "-"])
            .arg(app)
            .output()
        else {
            return false;
        };
        let info = String::from_utf8_lossy(&out.stderr);
        let entitlements = String::from_utf8_lossy(&out.stdout);
        info.contains("(runtime)")
            && !(entitlements.contains("allow-dyld-environment-variables")
                && entitlements.contains("disable-library-validation"))
    }
}

pub fn inspect(dir: &Path) -> Option<Game> {
    let files = entries(dir);
    let any = |ext: &str| files.iter().any(|p| has_ext(p, ext));
    let build = if files
        .iter()
        .any(|p| has_ext(p, "app") && p.join("Contents/MacOS").is_dir())
    {
        Build::MacOS
    } else if any("pck") && any("x86_64") {
        Build::Linux
    } else if any("pck") && any("exe") {
        if cfg!(windows) { Build::Windows } else { Build::Proton }
    } else {
        return None;
    };
    Some(Game {
        dir: dir.to_path_buf(),
        build,
    })
}

pub fn find_games() -> Vec<Game> {
    let mut games: Vec<Game> = Vec::new();
    for root in steam_roots() {
        let mut libraries = vec![root.clone()];
        if let Ok(vdf) = fs::read_to_string(root.join("steamapps/libraryfolders.vdf")) {
            libraries.extend(vdf_values(&vdf, "path").into_iter().map(PathBuf::from));
        }
        for library in libraries {
            let apps = library.join("steamapps");
            let folder = fs::read_to_string(apps.join(format!("appmanifest_{STEAM_APP_ID}.acf")))
                .ok()
                .and_then(|acf| vdf_values(&acf, "installdir").pop())
                .unwrap_or_else(|| GAME_FOLDER.into());
            let Some(game) = inspect(&apps.join("common").join(folder)) else {
                continue;
            };
            let real = fs::canonicalize(&game.dir).ok();
            if !games.iter().any(|g| fs::canonicalize(&g.dir).ok() == real) {
                games.push(game);
            }
        }
    }
    games
}

fn steam_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    #[cfg(windows)]
    {
        use winreg::RegKey;
        use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
        let keys = [
            (HKEY_CURRENT_USER, r"Software\Valve\Steam", "SteamPath"),
            (HKEY_LOCAL_MACHINE, r"SOFTWARE\WOW6432Node\Valve\Steam", "InstallPath"),
        ];
        for (hive, key, value) in keys {
            if let Ok(path) = RegKey::predef(hive)
                .open_subkey(key)
                .and_then(|k| k.get_value::<String, _>(value))
            {
                roots.push(PathBuf::from(path));
            }
        }
        roots.push(PathBuf::from(r"C:\Program Files (x86)\Steam"));
    }
    let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from);
    if let Some(home) = home {
        let candidates: &[&str] = if cfg!(target_os = "macos") {
            &["Library/Application Support/Steam"]
        } else if cfg!(windows) {
            &[]
        } else {
            &[
                ".local/share/Steam",
                ".steam/steam",
                ".var/app/com.valvesoftware.Steam/.local/share/Steam",
                "snap/steam/common/.local/share/Steam",
            ]
        };
        roots.extend(candidates.iter().map(|p| home.join(p)));
    }
    roots
}

fn vdf_values(text: &str, key: &str) -> Vec<String> {
    let key = format!("\"{key}\"");
    text.lines()
        .filter_map(|line| {
            let value = line
                .trim()
                .strip_prefix(&key)?
                .trim()
                .strip_prefix('"')?
                .strip_suffix('"')?;
            Some(value.replace("\\\\", "\\"))
        })
        .collect()
}

pub fn install(game: &Game, log: &dyn Fn(String)) -> Result<(), String> {
    let mods_dir = game.gdpatch_dir().join("mods");
    fs::create_dir_all(&mods_dir).map_err(|e| format!("Cannot create {}: {e}", mods_dir.display()))?;

    if game.gdpatch_installed() {
        log("GDPatch is already installed. Keeping it.".into());
    } else {
        let (path, asset) = game.loader();
        log(format!("Downloading GDPatch ({asset})..."));
        let (bytes, version) = download(&format!("{GDPATCH_RELEASE}/{asset}"))?;
        if path.exists() {
            let backup = path.with_file_name(format!("{}.bak", file_name(&path)));
            fs::rename(&path, &backup).map_err(|e| format!("Cannot back up {}: {e}", path.display()))?;
            log(format!(
                "Moved the existing {} to {}",
                file_name(&path),
                file_name(&backup)
            ));
        }
        write(&path, &bytes)?;
        log(format!("Installed GDPatch {version}."));
    }

    if game.build == Build::Linux {
        let script = game.dir.join("run_with_gdpatch.sh");
        if !script.exists() {
            log("Downloading run_with_gdpatch.sh...".into());
            write(&script, &download(RUN_SCRIPT_URL)?.0)?;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&script, fs::Permissions::from_mode(0o755))
                .map_err(|e| format!("Cannot make {} executable: {e}", script.display()))?;
        }
    }

    for (id, url) in MODS {
        log(format!("Downloading {id}..."));
        let (bytes, version) = download(url)?;
        install_mod(&mods_dir, id, bytes)?;
        log(format!("Installed {id} {version}."));
    }
    Ok(())
}

fn install_mod(mods_dir: &Path, id: &str, zip_bytes: Vec<u8>) -> Result<(), String> {
    let target = mods_dir.join(id);
    if target.exists() {
        fs::remove_dir_all(&target).map_err(|e| format!("Cannot remove the old {id}: {e}"))?;
    }
    zip::ZipArchive::new(Cursor::new(zip_bytes))
        .and_then(|mut zip| zip.extract(mods_dir))
        .map_err(|e| format!("{id} archive is broken: {e}"))?;
    if !target.join("gdpatch_mod.toml").exists() {
        return Err(format!("{id} archive does not contain {id}/gdpatch_mod.toml."));
    }
    Ok(())
}

fn download(url: &str) -> Result<(Vec<u8>, String), String> {
    use ureq::ResponseExt;
    let agent: ureq::Agent = ureq::Agent::config_builder().save_redirect_history(true).build().into();
    let mut res = agent.get(url).call().map_err(|e| match e {
        ureq::Error::StatusCode(404) => format!("{url} was not found. The release may not be published yet."),
        e => format!("Download failed ({url}): {e}"),
    })?;
    let tag = res
        .get_redirect_history()
        .unwrap_or_default()
        .iter()
        .find_map(|uri| {
            uri.path()
                .split("/releases/download/")
                .nth(1)?
                .split('/')
                .next()
                .map(String::from)
        })
        .unwrap_or_else(|| "(latest)".into());
    let bytes = res
        .body_mut()
        .with_config()
        .limit(256 * 1024 * 1024)
        .read_to_vec()
        .map_err(|e| format!("Download interrupted ({url}): {e}"))?;
    Ok((bytes, tag))
}

fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    fs::write(path, bytes).map_err(|e| format!("Cannot write {}: {e}", path.display()))
}

fn entries(dir: &Path) -> Vec<PathBuf> {
    fs::read_dir(dir)
        .map(|it| it.filter_map(|e| e.ok().map(|e| e.path())).collect())
        .unwrap_or_default()
}

fn has_ext(path: &Path, ext: &str) -> bool {
    path.extension().is_some_and(|e| e.eq_ignore_ascii_case(ext))
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

pub fn open(target: &str) {
    let program = if cfg!(windows) {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let _ = Command::new(program).arg(target).spawn();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_library_paths() {
        let vdf = "\"libraryfolders\"\n{\n\t\"0\"\n\t{\n\t\t\"path\"\t\t\"C:\\\\Program Files (x86)\\\\Steam\"\n\t}\n\t\"1\"\n\t{\n\t\t\"path\"\t\t\"/mnt/games/SteamLibrary\"\n\t}\n}";
        assert_eq!(
            vdf_values(vdf, "path"),
            ["C:\\Program Files (x86)\\Steam", "/mnt/games/SteamLibrary"]
        );
        assert_eq!(
            vdf_values("\t\"installdir\"\t\t\"Some Game\"", "installdir"),
            ["Some Game"]
        );
    }

    #[test]
    fn detects_builds() {
        let root = std::env::temp_dir().join(format!("avast-installer-test-{}", std::process::id()));
        let make = |name: &str, files: &[&str]| {
            let dir = root.join(name);
            for f in files {
                let p = dir.join(f);
                fs::create_dir_all(p.parent().unwrap()).unwrap();
                fs::write(p, b"").unwrap();
            }
            dir
        };
        let linux = make("linux", &["AVS03Pro.x86_64", "AVS03Pro.pck"]);
        let windows = make("windows", &["AVS03Pro.exe", "AVS03Pro.pck"]);
        let mac = make("mac", &["AVS03Pro.app/Contents/MacOS/AVS03Pro"]);
        let other = make("other", &["readme.txt"]);

        assert_eq!(inspect(&linux).unwrap().build, Build::Linux);
        let expected = if cfg!(windows) { Build::Windows } else { Build::Proton };
        assert_eq!(inspect(&windows).unwrap().build, expected);
        assert_eq!(inspect(&mac).unwrap().build, Build::MacOS);
        assert!(inspect(&other).is_none());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn replaces_mod_folder_from_zip() {
        use std::io::Write;
        let mods = std::env::temp_dir().join(format!("avast-installer-zip-{}", std::process::id()));
        fs::create_dir_all(mods.join("avast_api/data")).unwrap();
        fs::write(mods.join("avast_api/data/stale.gd"), b"old").unwrap();

        let mut buf = Cursor::new(Vec::new());
        let mut zip = zip::ZipWriter::new(&mut buf);
        let opts = zip::write::SimpleFileOptions::default();
        zip.start_file("avast_api/gdpatch_mod.toml", opts).unwrap();
        zip.write_all(b"id = \"avast_api\"\n").unwrap();
        zip.start_file("avast_api/data/avast/api/mod.gd", opts).unwrap();
        zip.write_all(b"extends Node\n").unwrap();
        zip.finish().unwrap();

        install_mod(&mods, "avast_api", buf.into_inner()).unwrap();
        assert!(mods.join("avast_api/data/avast/api/mod.gd").exists());
        assert!(!mods.join("avast_api/data/stale.gd").exists());
        assert!(install_mod(&mods, "avast_core", b"not a zip".to_vec()).is_err());
        let _ = fs::remove_dir_all(mods);
    }

    #[test]
    #[ignore]
    fn installs_into_fake_game() {
        let dir = std::env::temp_dir().join(format!("avast-installer-net-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("AVS03Pro.x86_64"), b"").unwrap();
        fs::write(dir.join("AVS03Pro.pck"), b"").unwrap();
        let game = inspect(&dir).unwrap();
        let result = install(&game, &|line| println!("{line}"));
        println!("{result:?}");
        assert!(game.gdpatch_installed());
        assert!(dir.join("run_with_gdpatch.sh").exists());
        let _ = fs::remove_dir_all(dir);
    }
}

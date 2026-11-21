use super::*;
use crate::browser::{details, download, modrinth, FileRef, SourceId};
use sha2::Digest;
use std::collections::HashMap;
use std::path::PathBuf;

/// A source in memory: projects, their versions (newest first), and the bytes each URL serves.
#[derive(Default)]
struct Fake {
    projects: HashMap<String, Project>,
    versions: HashMap<String, Vec<Version>>,
    bytes: HashMap<String, Vec<u8>>,
}

impl Source for Fake {
    fn id(&self) -> SourceId {
        SourceId::Modrinth
    }
    fn search(&self, _: &Query) -> Result<Vec<Project>, String> {
        Ok(self.projects.values().cloned().collect())
    }
    fn project(&self, id: &str) -> Result<Project, String> {
        self.projects
            .get(id)
            .cloned()
            .ok_or_else(|| "not found".into())
    }
    fn versions(&self, p: &Project) -> Result<Vec<Version>, String> {
        Ok(self.versions.get(&p.id).cloned().unwrap_or_default())
    }
    fn fetch(
        &self,
        file: &FileRef,
        folder: &Path,
        progress: &dyn Fn(f32),
    ) -> Result<PathBuf, String> {
        download::save(file, &self.bytes[&file.url][..], folder, progress)
    }
}

impl Fake {
    /// A new newest version of `id` (a project added on first use).
    fn release(
        &mut self,
        id: &str,
        kind: Kind,
        number: &str,
        games: &[&str],
        loaders: &[&str],
        needs: &[&str],
    ) {
        self.projects.entry(id.into()).or_insert_with(|| Project {
            source: SourceId::Modrinth,
            id: id.into(),
            slug: format!("{id}-slug"),
            title: id.into(),
            summary: String::new(),
            author: String::new(),
            downloads: 0,
            page: String::new(),
            icon: None,
            license: String::new(),
            kind,
            game_versions: Vec::new(),
            loaders: Vec::new(),
        });
        let body = format!("{id} {number}").into_bytes();
        let url = format!("https://cdn.modrinth.com/data/{id}/versions/{number}/{id}-{number}.zip");
        let file = FileRef {
            url: url.clone(),
            name: format!("{id}-{number}.zip"),
            size: body.len() as u64,
            sha512: Some(
                sha2::Sha512::digest(&body)
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect(),
            ),
            sha256: None,
            sha1: None,
            primary: true,
        };
        let strings = |l: &[&str]| l.iter().map(|s| s.to_string()).collect();
        self.bytes.insert(url, body);
        self.versions.entry(id.into()).or_default().insert(
            0,
            Version {
                id: format!("{id}@{number}"),
                number: number.into(),
                published: String::new(),
                game_versions: strings(games),
                loaders: strings(loaders),
                files: vec![file],
                required: strings(needs),
                blocked: None,
            },
        );
    }
}

fn target(minecraft: &str, _loader: Option<&str>) -> Target {
    Target {
        minecraft: minecraft.into(),
        loader: None,
    }
}

fn home(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "industrial_era-java-instance-{name}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

const NONE: &dyn Fn(&str, f32) = &|_, _| {};

#[test]
fn picks_the_newest_file_for_the_version_and_loader() {
    let mut fake = Fake::default();
    fake.release(
        "faithful",
        Kind::ResourcePack,
        "1",
        &["1.21.1"],
        &["minecraft"],
        &[],
    );
    fake.release(
        "faithful",
        Kind::ResourcePack,
        "2",
        &["1.21.4"],
        &["minecraft"],
        &[],
    );
    let versions = &fake.versions["faithful"];
    assert_eq!(
        target("1.21.1", None)
            .pick(Kind::ResourcePack, versions)
            .map(|v| v.number.as_str()),
        Some("1")
    );
    assert_eq!(
        target("1.21.4", None)
            .pick(Kind::ResourcePack, versions)
            .map(|v| v.number.as_str()),
        Some("2")
    );
    assert!(target("1.20.1", None)
        .pick(Kind::ResourcePack, versions)
        .is_none());
}

#[test]
fn searches_for_what_runs_in_the_instance() {
    let t = target("1.20.1", None);
    assert_eq!(
        modrinth::facets(&t.query("", Kind::ResourcePack)),
        r#"[["project_type:resourcepack"],["versions:1.20.1"]]"#
    );
}

#[test]
fn installs_jars_with_dependencies_then_updates_and_removes() {
    let home = home("cycle");
    let t = target("1.21.1", None);
    let mut fake = Fake::default();
    fake.release(
        "faithful",
        Kind::ResourcePack,
        "1",
        &["1.21.1"],
        &["minecraft"],
        &[],
    );
    fake.release(
        "sphax",
        Kind::ResourcePack,
        "1",
        &["1.21.1"],
        &["minecraft"],
        &["faithful"],
    );
    let sphax = fake.project("sphax").unwrap();

    let steps = plan(&fake, &t, &sphax).unwrap();
    let names: Vec<_> = steps
        .iter()
        .map(|s| (s.project.id.as_str(), s.dependency))
        .collect();
    assert_eq!(names, [("faithful", true), ("sphax", false)]);

    let done = install(&fake, &t, &sphax, &home, NONE).unwrap();
    assert_eq!(done.len(), 2);
    assert_eq!(
        fs::read(home.join("resourcepacks/sphax-1.zip")).unwrap(),
        b"sphax 1"
    );
    assert!(home.join("resourcepacks/faithful-1.zip").is_file());
    assert_eq!(Installed::load(&home).entries.len(), 2);

    fake.release(
        "sphax",
        Kind::ResourcePack,
        "2",
        &["1.21.1"],
        &["minecraft"],
        &[],
    );
    let entry = Installed::load(&home).lookup("sphax").cloned().unwrap();
    assert_eq!(update_for(&fake, &t, &entry).unwrap().unwrap().number, "2");
    install(&fake, &t, &sphax, &home, NONE).unwrap();
    assert!(!home.join("resourcepacks/sphax-1.zip").exists());
    assert_eq!(
        fs::read(home.join("resourcepacks/sphax-2.zip")).unwrap(),
        b"sphax 2"
    );
    let entry = Installed::load(&home).lookup("sphax").cloned().unwrap();
    assert!(update_for(&fake, &t, &entry).unwrap().is_none());

    remove(&home, "sphax-slug").unwrap();
    assert!(!home.join("resourcepacks/sphax-2.zip").exists());
    assert!(Installed::load(&home).lookup("sphax").is_none());
    assert!(home.join("resourcepacks/faithful-1.zip").is_file());
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn refuses_what_the_instance_cannot_run() {
    let home = home("refuse");
    let mut fake = Fake::default();
    fake.release(
        "faithful",
        Kind::ResourcePack,
        "1",
        &["1.21.1"],
        &["minecraft"],
        &[],
    );
    let err = install(
        &fake,
        &target("1.20.1", None),
        &fake.project("faithful").unwrap(),
        &home,
        NONE,
    )
    .unwrap_err();
    assert_eq!(err, "faithful has no version for Minecraft 1.20.1.");
    assert!(!home.join("resourcepacks").exists());
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn the_page_search_keeps_the_players_filters_but_not_their_version() {
    let t = target("1.21.1", None);
    let picked = details::Filters {
        categories: vec!["optimization".into()],
        loader: None,
        game_version: Some("26.3".into()),
        environment: Some(details::Environment::Client),
        sort: Some(details::Sort::Downloads),
    };
    let q = t.search("fps", Kind::ResourcePack, &picked);
    assert_eq!(q.text, "fps");
    assert_eq!(
        q.filters,
        details::Filters {
            game_version: Some("1.21.1".into()),
            loader: None,
            ..picked
        }
    );
    assert!(t.takes(Kind::ResourcePack));
    assert_eq!(t.describe(Kind::ResourcePack), "Minecraft 1.21.1");
}

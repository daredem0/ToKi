use super::{collect_source_files, export_hybrid_bundle};
use crate::project::Project;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use toki_core::pack::{PackAssetType, PackCompression, PakManifest, PAK_MAGIC, PAK_VERSION};
use toki_core::project_runtime::RuntimeConfigFile;

#[test]
fn collect_source_files_returns_sorted_relative_paths() {
    let temp = tempfile::tempdir().expect("temp dir");
    let project_root = temp.path().join("MyGame");
    fs::create_dir_all(project_root.join("assets/sprites")).expect("assets dir");
    fs::create_dir_all(project_root.join("scenes")).expect("scenes dir");
    fs::write(project_root.join("project.toml"), "name = 'MyGame'").expect("project");
    fs::write(project_root.join("scenes/main.json"), "{}").expect("scene");
    fs::write(project_root.join("assets/sprites/a.png"), "a").expect("asset");

    let files = collect_source_files(&project_root, None).expect("collect files");
    let relative = files
        .iter()
        .map(|f| f.relative_path.to_string_lossy().to_string())
        .collect::<Vec<_>>();
    assert_eq!(
        relative,
        vec![
            "assets/sprites/a.png".to_string(),
            "project.toml".to_string(),
            "scenes/main.json".to_string(),
        ]
    );
}

#[test]
fn export_hybrid_bundle_writes_runtime_and_pak_manifest() {
    let temp = tempfile::tempdir().expect("temp dir");
    let parent = temp.path();
    let project_root = parent.join("MyGame");
    fs::create_dir_all(project_root.join("assets/audio")).expect("assets dir");
    fs::create_dir_all(project_root.join("scenes")).expect("scenes dir");
    fs::write(project_root.join("project.toml"), "name = 'MyGame'").expect("project");
    fs::write(project_root.join("scenes/main.json"), "{\"name\":\"main\"}").expect("scene");
    fs::write(project_root.join("assets/audio/test.ogg"), "audio").expect("audio");

    let runtime_bin = parent.join(if cfg!(target_os = "windows") {
        "toki-runtime.exe"
    } else {
        "toki-runtime"
    });
    fs::write(&runtime_bin, "runtime-binary").expect("runtime");

    let project = Project::new("MyGame".to_string(), project_root.clone());
    let mut project = project;
    project.metadata.runtime.audio.master_percent = 80;
    project.metadata.runtime.audio.music_percent = 65;
    project.metadata.runtime.audio.movement_percent = 40;
    project.metadata.runtime.audio.collision_percent = 25;
    project.metadata.runtime.display.show_entity_health_bars = true;
    let export_root = parent.join("exports");
    fs::create_dir_all(&export_root).expect("exports dir");

    let bundle_dir = export_hybrid_bundle(
        &project,
        &runtime_bin,
        &export_root,
        Some("Main Scene"),
        3000,
    )
    .expect("bundle export");
    assert!(bundle_dir.join(runtime_bin.file_name().unwrap()).exists());
    assert!(!bundle_dir.join("project.toml").exists());
    assert!(!bundle_dir.join("scenes/main.json").exists());
    assert!(!bundle_dir.join("assets/audio/test.ogg").exists());
    let pak_path = bundle_dir.join("game.toki.pak");
    assert!(pak_path.exists());
    let config_path = bundle_dir.join("runtime_config.json");
    assert!(config_path.exists());
    let toki_license_path = bundle_dir.join("LICENSE-TOKI.md");
    assert!(toki_license_path.exists());
    let third_party_licenses_path = bundle_dir.join("THIRD_PARTY_LICENSES.md");
    assert!(third_party_licenses_path.exists());

    let root_entries = fs::read_dir(&bundle_dir)
        .expect("read bundle root")
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect::<Vec<_>>();
    assert_eq!(root_entries.len(), 5);
    assert!(root_entries.contains(
        &runtime_bin
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string()
    ));
    assert!(root_entries.contains(&"game.toki.pak".to_string()));
    assert!(root_entries.contains(&"runtime_config.json".to_string()));
    assert!(root_entries.contains(&"LICENSE-TOKI.md".to_string()));
    assert!(root_entries.contains(&"THIRD_PARTY_LICENSES.md".to_string()));

    let mut pak_file = fs::File::open(&pak_path).expect("open pak");
    let mut magic = [0u8; 8];
    pak_file.read_exact(&mut magic).expect("read magic");
    assert_eq!(&magic, PAK_MAGIC);

    let mut index_offset_buf = [0u8; 8];
    let mut index_size_buf = [0u8; 8];
    pak_file
        .read_exact(&mut index_offset_buf)
        .expect("read offset");
    pak_file.read_exact(&mut index_size_buf).expect("read size");
    let index_offset = u64::from_le_bytes(index_offset_buf);
    let index_size = u64::from_le_bytes(index_size_buf);
    assert!(index_offset > 0);
    assert!(index_size > 0);

    pak_file
        .seek(SeekFrom::Start(index_offset))
        .expect("seek index");
    let mut index_bytes = vec![0u8; index_size as usize];
    pak_file.read_exact(&mut index_bytes).expect("read index");
    let manifest: PakManifest = serde_json::from_slice(&index_bytes).expect("manifest");
    assert_eq!(manifest.version, PAK_VERSION);
    assert!(manifest
        .entries
        .iter()
        .any(|entry| entry.path == "project.toml"
            && entry.asset_type == PackAssetType::ProjectConfig
            && entry.hash.is_some()
            && entry.stored_size > 0));
    assert!(manifest
        .entries
        .iter()
        .any(|entry| entry.path == "scenes/main.json"
            && entry.asset_type == PackAssetType::Scene
            && entry.hash.is_some()
            && entry.stored_size > 0));
    assert!(manifest
        .entries
        .iter()
        .any(|entry| entry.path == "assets/audio/test.ogg"
            && entry.compression == PackCompression::Store
            && entry.asset_type == PackAssetType::Audio
            && entry.hash.is_some()
            && entry.stored_size == entry.size));

    let runtime_config: RuntimeConfigFile =
        serde_json::from_str(&fs::read_to_string(config_path).expect("read config"))
            .expect("parse config");
    assert_eq!(runtime_config.version, 1);
    assert_eq!(
        runtime_config.pack.as_ref().map(|pack| pack.path.as_str()),
        Some("game.toki.pak")
    );
    assert_eq!(
        runtime_config.pack.as_ref().map(|pack| pack.enabled),
        Some(true)
    );
    assert_eq!(
        runtime_config
            .startup
            .as_ref()
            .and_then(|startup| startup.scene.as_deref()),
        Some("Main Scene")
    );
    assert_eq!(
        runtime_config
            .splash
            .as_ref()
            .and_then(|splash| splash.duration_ms),
        Some(3000)
    );
    assert_eq!(
        runtime_config
            .audio
            .as_ref()
            .and_then(|audio| audio.master_percent),
        Some(80)
    );
    assert_eq!(
        runtime_config
            .audio
            .as_ref()
            .and_then(|audio| audio.music_percent),
        Some(65)
    );
    assert_eq!(
        runtime_config
            .audio
            .as_ref()
            .and_then(|audio| audio.movement_percent),
        Some(40)
    );
    assert_eq!(
        runtime_config
            .audio
            .as_ref()
            .and_then(|audio| audio.collision_percent),
        Some(25)
    );
    assert_eq!(
        runtime_config
            .display
            .as_ref()
            .and_then(|display| display.show_entity_health_bars),
        Some(true)
    );
    assert_eq!(
        runtime_config
            .menu
            .as_ref()
            .map(|menu| menu.pause_root_screen_id.as_str()),
        Some("pause_menu")
    );
    assert_eq!(
        runtime_config
            .menu
            .as_ref()
            .map(|menu| menu.gate_gameplay_when_open),
        Some(true)
    );
    assert_eq!(
        runtime_config.menu.as_ref().map(|menu| menu.screens.len()),
        Some(2)
    );
}

#[test]
fn export_hybrid_bundle_uses_safe_bundle_directory_suffix() {
    let temp = tempfile::tempdir().expect("temp dir");
    let parent = temp.path();
    let project_root = parent.join("MyGame");
    fs::create_dir_all(project_root.join("assets")).expect("assets");
    fs::write(project_root.join("project.toml"), "name='MyGame'").expect("project");
    fs::write(project_root.join("assets/file.txt"), "payload").expect("asset");

    let runtime_bin = parent.join(if cfg!(target_os = "windows") {
        "toki-runtime.exe"
    } else {
        "toki-runtime"
    });
    fs::write(&runtime_bin, "runtime-binary").expect("runtime");

    let project = Project::new("MyGame".to_string(), project_root.clone());
    let bundle_dir = export_hybrid_bundle(&project, &runtime_bin, parent, Some("Main Scene"), 3000)
        .expect("bundle export");

    assert_eq!(bundle_dir, parent.join("MyGame-bundle"));
    assert!(project_root.join("project.toml").exists());
    assert!(project_root.join("assets/file.txt").exists());
}

#[test]
fn export_hybrid_bundle_compresses_text_assets_and_stores_already_compressed_assets() {
    let temp = tempfile::tempdir().expect("temp dir");
    let parent = temp.path();
    let project_root = parent.join("MyGame");
    fs::create_dir_all(project_root.join("assets/sprites")).expect("sprites dir");
    fs::create_dir_all(project_root.join("scenes")).expect("scenes dir");
    fs::write(
        project_root.join("project.toml"),
        "name='MyGame'\nversion='1'",
    )
    .expect("project");
    let repeated_entities = (0..128)
        .map(|index| format!("{{\"id\":{index},\"type\":\"npc\",\"x\":0,\"y\":0}}"))
        .collect::<Vec<_>>()
        .join(",");
    fs::write(
        project_root.join("scenes/main.json"),
        format!("{{\"name\":\"main\",\"maps\":[],\"entities\":[{repeated_entities}]}}"),
    )
    .expect("scene");
    fs::write(
        project_root.join("assets/sprites/a.png"),
        [137u8, 80, 78, 71, 13, 10, 26, 10],
    )
    .expect("png");

    let runtime_bin = parent.join(if cfg!(target_os = "windows") {
        "toki-runtime.exe"
    } else {
        "toki-runtime"
    });
    fs::write(&runtime_bin, "runtime-binary").expect("runtime");

    let project = Project::new("MyGame".to_string(), project_root.clone());
    let bundle_dir = export_hybrid_bundle(&project, &runtime_bin, parent, Some("Main Scene"), 3000)
        .expect("bundle export");

    let pak_path = bundle_dir.join("game.toki.pak");
    let mut pak_file = fs::File::open(&pak_path).expect("open pak");
    pak_file.seek(SeekFrom::Start(8)).expect("seek header");
    let mut index_offset_buf = [0u8; 8];
    let mut index_size_buf = [0u8; 8];
    pak_file.read_exact(&mut index_offset_buf).expect("offset");
    pak_file.read_exact(&mut index_size_buf).expect("size");
    let index_offset = u64::from_le_bytes(index_offset_buf);
    let index_size = u64::from_le_bytes(index_size_buf);
    pak_file
        .seek(SeekFrom::Start(index_offset))
        .expect("seek index");
    let mut index_bytes = vec![0u8; index_size as usize];
    pak_file.read_exact(&mut index_bytes).expect("read index");
    let manifest: PakManifest = serde_json::from_slice(&index_bytes).expect("manifest");

    let scene_entry = manifest
        .entries
        .iter()
        .find(|entry| entry.path == "scenes/main.json")
        .expect("scene entry");
    assert_eq!(scene_entry.compression, PackCompression::Zstd);
    assert!(scene_entry.stored_size <= scene_entry.size);

    let image_entry = manifest
        .entries
        .iter()
        .find(|entry| entry.path == "assets/sprites/a.png")
        .expect("image entry");
    assert_eq!(image_entry.compression, PackCompression::Store);
    assert_eq!(image_entry.stored_size, image_entry.size);
}

use std::error::Error;
use std::io;
use std::path::PathBuf;
use toki_core::CoreError;

#[test]
fn error_file_load_displays_correctly() {
    let path = PathBuf::from("test.json");
    let message = "File corrupted";
    let error = CoreError::FileLoad(path, message.to_string());

    let error_string = format!("{}", error);
    assert!(error_string.contains("Failed to load file at test.json"));
    assert!(error_string.contains("File corrupted"));
}

#[test]
fn error_image_load_displays_correctly() {
    let message = "Invalid PNG format";
    let error = CoreError::ImageLoad(message.to_string());

    let error_string = format!("{}", error);
    assert_eq!(error_string, "Image load failed: Invalid PNG format");
}

#[test]
fn error_io_conversion_works() {
    let io_error = io::Error::new(io::ErrorKind::NotFound, "File not found");
    let core_error: CoreError = io_error.into();

    match core_error {
        CoreError::Io(_) => {
            // Expected
            let error_string = format!("{}", core_error);
            assert!(error_string.contains("I/O error while reading file"));
            assert!(error_string.contains("File not found"));
        }
        _ => panic!("Expected CoreError::Io variant"),
    }
}

#[test]
fn error_json_conversion_works() {
    let json_str = r#"{"invalid": json syntax"#; // Missing closing brace
    let json_result = serde_json::from_str::<serde_json::Value>(json_str);

    assert!(json_result.is_err());
    let json_error = json_result.unwrap_err();
    let core_error: CoreError = json_error.into();

    match core_error {
        CoreError::Json(_) => {
            // Expected
            let error_string = format!("{}", core_error);
            assert!(error_string.contains("Invalid atlas JSON"));
        }
        _ => panic!("Expected CoreError::Json variant"),
    }
}

#[test]
fn error_not_found_displays_correctly() {
    let path = PathBuf::from("/missing/atlas.json");
    let error = CoreError::NotFound(path);

    let error_string = format!("{}", error);
    assert_eq!(error_string, "Atlas file not found: /missing/atlas.json");
}

#[test]
fn error_invalid_map_size_displays_correctly() {
    let error = CoreError::InvalidMapSize {
        expected: 16,
        actual: 12,
    };

    let error_string = format!("{}", error);
    assert_eq!(
        error_string,
        "Map size mismatch: expected 16 tiles, found 12"
    );
}

#[test]
fn error_debug_format_works() {
    let error = CoreError::ImageLoad("Test error".to_string());
    let debug_string = format!("{:?}", error);

    assert!(debug_string.contains("ImageLoad"));
    assert!(debug_string.contains("Test error"));
}

#[test]
fn error_chain_io_to_core() {
    // Simulate reading a nonexistent file
    let io_result = std::fs::read_to_string("definitely_nonexistent_file.txt");
    assert!(io_result.is_err());

    let io_error = io_result.unwrap_err();
    let core_error: CoreError = io_error.into();

    // Should preserve the original IO error information
    match core_error {
        CoreError::Io(inner) => {
            assert_eq!(inner.kind(), io::ErrorKind::NotFound);
        }
        _ => panic!("Expected CoreError::Io"),
    }
}

#[test]
fn error_different_io_kinds() {
    let permission_error = io::Error::new(io::ErrorKind::PermissionDenied, "Access denied");
    let core_error: CoreError = permission_error.into();

    if let CoreError::Io(inner) = core_error {
        assert_eq!(inner.kind(), io::ErrorKind::PermissionDenied);
        let error_string = format!("{}", CoreError::Io(inner));
        assert!(error_string.contains("Access denied"));
    } else {
        panic!("Expected CoreError::Io");
    }
}

#[test]
fn error_send_sync_traits() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<CoreError>();
    assert_sync::<CoreError>();
}

#[test]
fn error_std_error_trait() {
    let error = CoreError::ImageLoad("test".to_string());

    // Should implement std::error::Error
    fn assert_error<T: std::error::Error>(_: &T) {}
    assert_error(&error);

    // Should have a source method (from std::error::Error trait)
    assert!(error.source().is_none()); // ImageLoad doesn't have a source

    // Test an error that does have a source
    let io_error = io::Error::new(io::ErrorKind::NotFound, "test");
    let core_error: CoreError = io_error.into();

    if let CoreError::Io(_) = core_error {
        assert!(core_error.source().is_some());
    }
}

#[test]
fn error_invalid_map_size_with_zero() {
    let error = CoreError::InvalidMapSize {
        expected: 0,
        actual: 5,
    };

    let error_string = format!("{}", error);
    assert_eq!(error_string, "Map size mismatch: expected 0 tiles, found 5");
}

#[test]
fn error_path_with_unicode() {
    let path = PathBuf::from("files/测试.json");
    let error = CoreError::NotFound(path);

    let error_string = format!("{}", error);
    assert!(error_string.contains("files/测试.json"));
}

#[test]
fn error_equality_and_cloning() {
    // Note: CoreError doesn't derive PartialEq or Clone due to io::Error
    // But we can test that it properly handles different error types

    let error1 = CoreError::ImageLoad("test1".to_string());
    let error2 = CoreError::ImageLoad("test2".to_string());

    // They should be different errors with different messages
    assert_ne!(format!("{}", error1), format!("{}", error2));
}

#[test]
fn error_from_json_with_complex_error() {
    // Create a more complex JSON parsing error
    let json_str = r#"{"map": {"size": "invalid_number"}}"#;
    let json_result = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(json_str);

    // This should succeed (the JSON is valid)
    assert!(json_result.is_ok());

    // Try parsing as a struct that expects a number
    #[derive(serde::Deserialize, Debug)]
    struct TestStruct {
        #[allow(dead_code)]
        map: TestMap,
    }

    #[derive(serde::Deserialize, Debug)]
    struct TestMap {
        #[allow(dead_code)]
        size: u32,
    }

    let struct_result = serde_json::from_str::<TestStruct>(json_str);
    assert!(struct_result.is_err());

    let json_error = struct_result.unwrap_err();
    let core_error: CoreError = json_error.into();

    match core_error {
        CoreError::Json(_) => {
            let error_string = format!("{}", core_error);
            assert!(error_string.contains("Invalid atlas JSON"));
        }
        _ => panic!("Expected CoreError::Json variant"),
    }
}

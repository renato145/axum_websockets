use crate::helpers::spawn_app;
use actix_websockets::websocket::message::WebsocketSystems;

#[actix_rt::test]
async fn get_files_receive_python_files_on_valid_path() {
    // Arrange
    let app = spawn_app().await;
    let message = serde_json::json!({
        "system": "python_repo",
        "task": "get_files",
        "payload": "tests/examples"

    })
    .to_string();

    // Act
    let result = app.get_first_result(&message).await;

    // Assert
    assert_eq!(result.system.unwrap(), WebsocketSystems::PythonRepo);
    assert!(result.success, "Call was not successful.");
    let payload = result.payload.to_string();
    assert!(
        payload.contains("a.py"),
        "Expected file (a.py) not found in payload."
    );
}

#[actix_rt::test]
async fn get_files_receive_error_on_invalid_path() {
    // Arrange
    let app = spawn_app().await;
    let message = serde_json::json!({
        "system": "python_repo",
        "task": "get_files",
        "payload": "tests/some_incorrect_path"
    })
    .to_string();

    // Act
    let result = app.get_first_result(&message).await;

    // Assert
    assert_eq!(result.system.unwrap(), WebsocketSystems::PythonRepo);
    assert!(!result.success, "Call should not success.");
}

#[actix_rt::test]
async fn receive_error_on_invalid_task_name() {
    // Arrange
    let app = spawn_app().await;
    let message = serde_json::json!({
        "system": "python_repo",
        "task": "invalid_task_name"
    })
    .to_string();

    // Act
    let result = app.get_first_result(&message).await;

    // Assert
    assert_eq!(result.system.unwrap(), WebsocketSystems::PythonRepo);
    assert!(!result.success, "Call should not success.");
}

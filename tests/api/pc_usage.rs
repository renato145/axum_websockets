use crate::helpers::spawn_app;
use actix_websockets::websocket::{message::WebsocketSystems, pc_usage::CpuLoadResult};

#[actix_rt::test]
async fn cpu_load_receives_results() {
    // Arrange
    let app = spawn_app().await;
    let message = serde_json::json!({
        "system": "pc_usage",
        "task":  "cpu_load",

    })
    .to_string();

    // Act
    let result = app.get_first_result(&message).await;

    // Assert
    assert_eq!(result.system.unwrap(), WebsocketSystems::PcUsage);
    assert!(result.success, "Call was not successful.");
    let payload = serde_json::from_value::<Vec<CpuLoadResult>>(result.payload)
        .expect("Failed to deserialize result.");
    assert!(payload.len() > 0, "Empty results.");
}

#[actix_rt::test]
async fn receive_error_on_invalid_task_name() {
    // Arrange
    let app = spawn_app().await;
    let message = serde_json::json!({
        "system": "pc_usage",
        "task":  "invalid_task_name",
    })
    .to_string();

    // Act
    let result = app.get_first_result(&message).await;

    // Assert
    assert_eq!(result.system.unwrap(), WebsocketSystems::PcUsage);
    assert!(!result.success, "Call should not success.");
}

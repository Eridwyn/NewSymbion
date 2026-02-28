//! System metrics command handler

use std::future::Future;
use std::pin::Pin;

use chrono::Utc;
use serde_json::Value;

use crate::execution::handler::{CommandHandler, CommandResult};
use crate::metrics;

pub struct MetricsHandler;

impl CommandHandler for MetricsHandler {
    fn command_types(&self) -> &[&str] {
        &["get_metrics"]
    }

    fn execute<'a>(
        &'a self,
        _command_type: &'a str,
        _params: Option<&'a Value>,
    ) -> Pin<Box<dyn Future<Output = CommandResult> + Send + 'a>> {
        Box::pin(async move {
            match metrics::SystemMetrics::collect().await {
                Ok(system_metrics) => {
                    let process_info = metrics::ProcessInfo::collect().await.ok();
                    let services = metrics::ServiceStatus::collect_critical().await.ok();
                    CommandResult::success(serde_json::json!({
                        "system": system_metrics,
                        "processes": process_info,
                        "services": services,
                        "timestamp": Utc::now()
                    }))
                }
                Err(e) => CommandResult::error("METRICS_ERROR", e.to_string()),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_metrics() {
        let handler = MetricsHandler;
        let result = handler.execute("get_metrics", None).await;
        assert_eq!(result.status, "success");
        let data = result.data.unwrap();
        assert!(data.get("system").is_some());
    }
}

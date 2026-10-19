mod common;
use common::Harness;
use lapis_acp_protocol::{Error, SessionHandler, SessionUpdate, TurnBudget};
use serde_json::json;
use std::time::Duration;

struct BlockedDelivery(usize);
impl SessionHandler for BlockedDelivery {
    fn session_update_async(
        &mut self,
        _update: SessionUpdate,
    ) -> impl std::future::Future<Output = ()> + Send {
        self.0 += 1;
        std::future::pending()
    }
}

#[tokio::test]
async fn awaiting_update_delivery_remains_inside_the_original_absolute_budget() {
    let mut harness = Harness::new();
    harness
        .send(json!({"jsonrpc":"2.0","method":"session/update","params":{
        "sessionId":"s","update":{"sessionUpdate":"agent_message_chunk",
        "content":{"type":"text","text":"bounded"}}}}))
        .await;
    let mut handler = BlockedDelivery(0);
    let budget = Duration::from_millis(50);
    let result = harness
        .connection
        .call_within(
            &mut handler,
            "session/prompt",
            json!({"sessionId":"s","prompt":[]}),
            TurnBudget::fixed(budget),
        )
        .await;
    assert!(matches!(result, Err(Error::Timeout(value)) if value == budget));
    assert_eq!(handler.0, 1);
}

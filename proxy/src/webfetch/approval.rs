use common::models::PendingToolInfo;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;

/// User decision for a pending webfetch tool call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalDecision {
    Fail,
    Mock,
    Accept,
}

/// A pending approval waiting for user action.
pub struct PendingApproval {
    pub session_id: String,
    pub tools: Vec<PendingToolInfo>,
    pub sender: oneshot::Sender<ApprovalDecision>,
}

/// Shared approval queue: maps approval_id → PendingApproval.
pub type ApprovalQueue = Arc<Mutex<HashMap<String, PendingApproval>>>;

/// Create a new empty approval queue.
pub fn new_approval_queue() -> ApprovalQueue {
    Arc::new(Mutex::new(HashMap::new()))
}

/// List pending approvals for a given session.
pub fn list_pending(
    queue: &ApprovalQueue,
    session_id: &str,
) -> Vec<(String, Vec<PendingToolInfo>)> {
    let queue_map = queue.lock().unwrap();
    queue_map
        .iter()
        .filter(|(_, pending)| pending.session_id == session_id)
        .map(|(id, pending)| (id.clone(), pending.tools.clone()))
        .collect()
}

/// Resolve a pending approval by sending the decision through the oneshot channel.
/// Returns `true` if the approval was found and resolved.
pub fn resolve_pending(
    queue: &ApprovalQueue,
    approval_id: &str,
    decision: ApprovalDecision,
) -> bool {
    let pending = {
        let mut queue_map = queue.lock().unwrap();
        queue_map.remove(approval_id)
    };
    if let Some(pending) = pending {
        let _ = pending.sender.send(decision);
        true
    } else {
        false
    }
}

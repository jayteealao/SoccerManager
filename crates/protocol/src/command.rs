//! The change queue. A client queues a tactical change or a substitution; the server holds
//! it in order and answers. Nothing is applied to play yet: the engine announces every
//! stoppage through its stoppage hook, and a change will apply at a stoppage the rule pack
//! admits it at.

use serde::{Deserialize, Serialize};

/// What a queued change does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChangeKind {
    Tactics,
    Substitution,
}

impl ChangeKind {
    /// Every kind, in declaration order.
    pub const ALL: [ChangeKind; 2] = [ChangeKind::Tactics, ChangeKind::Substitution];

    /// The kind as a client writes it.
    pub fn code(&self) -> &'static str {
        match self {
            ChangeKind::Tactics => "tactics",
            ChangeKind::Substitution => "substitution",
        }
    }

    /// The kind named by `code`, or `None`.
    pub fn parse(code: &str) -> Option<Self> {
        ChangeKind::ALL.into_iter().find(|k| k.code() == code)
    }
}

/// Where a queued change stands. The four words are the viewer's chip labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChangeState {
    Queued,
    AppliesNow,
    Applied,
    Rejected,
}

impl ChangeState {
    /// The word a viewer chip carries. Colour never carries the state alone.
    pub fn label(&self) -> &'static str {
        match self {
            ChangeState::Queued => "Queued",
            ChangeState::AppliesNow => "Applies now",
            ChangeState::Applied => "Applied",
            ChangeState::Rejected => "Rejected",
        }
    }
}

/// One change the queue holds until it applies at a stoppage.
#[derive(Debug, Clone, PartialEq)]
pub struct Pending {
    pub queue_id: String,
    pub kind: ChangeKind,
    pub queued_tick: u32,
    pub detail: serde_json::Value,
}

/// The server's answer to a client command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ack {
    /// The command this answers: `start`, `pause`, `set-speed`, or `queue-change`.
    pub command: String,
    #[serde(rename = "change.queue_id", skip_serializing_if = "Option::is_none")]
    pub queue_id: Option<String>,
    #[serde(rename = "change.queued_tick")]
    pub queued_tick: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<ChangeState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<f32>,
}

/// The server's refusal, naming the reason in words a viewer can show.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reject {
    pub command: String,
    pub reason: String,
}

/// The verdict on one queued change.
#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    Ack(Ack),
    Reject(Reject),
}

/// Pending changes in the order the client queued them.
#[derive(Debug, Clone)]
pub struct Queue {
    admitted: Vec<ChangeKind>,
    pending: Vec<Pending>,
    next: u32,
}

impl Queue {
    /// A queue that admits exactly the kinds the loaded rule pack allows at some stoppage.
    pub fn new(admitted: Vec<ChangeKind>) -> Self {
        Self {
            admitted,
            pending: Vec::new(),
            next: 0,
        }
    }

    /// The changes waiting, oldest first.
    pub fn pending(&self) -> &[Pending] {
        &self.pending
    }

    /// Queues one change, or refuses it naming the type. The identifier is
    /// `q-{tick}-{n}`, with `n` counting every submission this match.
    pub fn submit(&mut self, kind: &str, detail: serde_json::Value, tick: u32) -> Verdict {
        let Some(kind) = ChangeKind::parse(kind) else {
            return Verdict::Reject(Reject {
                command: "queue-change".into(),
                reason: format!("unknown change type {kind}"),
            });
        };
        if !self.admitted.contains(&kind) {
            return Verdict::Reject(Reject {
                command: "queue-change".into(),
                reason: format!("the rule pack admits no {} change", kind.code()),
            });
        }
        let queue_id = format!("q-{tick}-{}", self.next);
        self.next += 1;
        self.pending.push(Pending {
            queue_id: queue_id.clone(),
            kind,
            queued_tick: tick,
            detail,
        });
        Verdict::Ack(Ack {
            command: "queue-change".into(),
            queue_id: Some(queue_id),
            queued_tick: tick,
            state: Some(ChangeState::Queued),
            speed: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn queue() -> Queue {
        Queue::new(ChangeKind::ALL.to_vec())
    }

    #[test]
    fn a_valid_change_is_queued_with_an_identifier() {
        let mut q = queue();
        let Verdict::Ack(ack) = q.submit("tactics", serde_json::Value::Null, 1_200) else {
            panic!("a tactics change must be admitted");
        };
        assert_eq!(ack.queue_id.as_deref(), Some("q-1200-0"));
        assert_eq!(ack.queued_tick, 1_200);
        assert_eq!(ack.state, Some(ChangeState::Queued));
        assert_eq!(q.pending().len(), 1);
    }

    #[test]
    fn an_unknown_change_type_is_refused_naming_the_type() {
        let mut q = queue();
        let Verdict::Reject(reject) = q.submit("formation", serde_json::Value::Null, 7) else {
            panic!("an unknown type must be refused");
        };
        assert_eq!(reject.reason, "unknown change type formation");
        assert!(q.pending().is_empty());
    }

    #[test]
    fn a_kind_the_rule_pack_forbids_is_refused() {
        let mut q = Queue::new(vec![ChangeKind::Tactics]);
        let Verdict::Reject(reject) = q.submit("substitution", serde_json::Value::Null, 1) else {
            panic!("a forbidden kind must be refused");
        };
        assert_eq!(reject.reason, "the rule pack admits no substitution change");
    }

    #[test]
    fn every_state_carries_its_viewer_word() {
        assert_eq!(ChangeState::Queued.label(), "Queued");
        assert_eq!(ChangeState::AppliesNow.label(), "Applies now");
        assert_eq!(ChangeState::Applied.label(), "Applied");
        assert_eq!(ChangeState::Rejected.label(), "Rejected");
        assert_eq!(
            serde_json::to_string(&ChangeState::AppliesNow).unwrap(),
            "\"applies-now\""
        );
    }
}

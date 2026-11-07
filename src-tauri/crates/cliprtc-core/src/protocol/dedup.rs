use std::collections::{HashSet, VecDeque};

use uuid::Uuid;

use crate::protocol::ClipboardEvent;

#[derive(Debug)]
pub struct DedupWindow {
    boot_id: Option<Uuid>,
    last_seq: u64,
    seen: HashSet<Uuid>,
    order: VecDeque<Uuid>,
    capacity: usize,
}

impl DedupWindow {
    pub fn new(capacity: usize) -> Self {
        Self {
            boot_id: None,
            last_seq: 0,
            seen: HashSet::new(),
            order: VecDeque::new(),
            capacity: capacity.max(1),
        }
    }

    pub fn accept(&mut self, event: &ClipboardEvent) -> bool {
        if self.boot_id != Some(event.boot_id) {
            self.boot_id = Some(event.boot_id);
            self.last_seq = 0;
            self.seen.clear();
            self.order.clear();
        }

        if self.seen.contains(&event.event_id) {
            return false;
        }

        if event.seq.saturating_add(self.capacity as u64) <= self.last_seq {
            return false;
        }

        self.seen.insert(event.event_id);
        self.order.push_back(event.event_id);
        self.last_seq = self.last_seq.max(event.seq);

        while self.order.len() > self.capacity {
            if let Some(old) = self.order.pop_front() {
                self.seen.remove(&old);
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{ClipboardEvent, EventKind};

    fn event(boot: Uuid, seq: u64) -> ClipboardEvent {
        ClipboardEvent::new(Uuid::new_v4(), boot, seq, 1, EventKind::Text, [1; 32], 1)
    }

    #[test]
    fn accepts_once_and_rejects_duplicate() {
        let mut dedup = DedupWindow::new(8);
        let e = event(Uuid::new_v4(), 1);
        assert!(dedup.accept(&e));
        assert!(!dedup.accept(&e));
    }

    #[test]
    fn new_boot_resets_window() {
        let mut dedup = DedupWindow::new(8);
        let boot = Uuid::new_v4();
        assert!(dedup.accept(&event(boot, 10)));

        let new_boot = Uuid::new_v4();
        assert!(dedup.accept(&event(new_boot, 1)));
        assert!(dedup.accept(&event(new_boot, 10)));
    }

    #[test]
    fn rejects_seq_outside_window() {
        let mut dedup = DedupWindow::new(2);
        let boot = Uuid::new_v4();
        assert!(dedup.accept(&event(boot, 10)));
        assert!(!dedup.accept(&event(boot, 1)));
    }
}

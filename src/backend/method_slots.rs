use std::collections::HashMap;

#[derive(Default)]
pub struct MethodSlotRegistry {
    slots: HashMap<String, u32>,
    names: HashMap<u32, String>,
    next_slot: u32,
}

impl MethodSlotRegistry {
    pub fn new() -> Self {
        Self {
            slots: HashMap::new(),
            names: HashMap::new(),
            next_slot: 1,
        }
    }

    pub fn register(&mut self, method_name: &str) -> u32 {
        if let Some(&slot) = self.slots.get(method_name) {
            return slot;
        }
        let slot = self.next_slot;
        self.slots.insert(method_name.to_string(), slot);
        self.names.insert(slot, method_name.to_string());
        self.next_slot += 1;
        slot
    }

    pub fn get(&self, method_name: &str) -> Option<u32> {
        self.slots.get(method_name).copied()
    }

    pub fn name_for_slot(&self, slot: u32) -> Option<&str> {
        self.names.get(&slot).map(|s| s.as_str())
    }

    pub fn total_slots(&self) -> u32 {
        self.next_slot
    }
}

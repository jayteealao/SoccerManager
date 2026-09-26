//! The golden-file guard: the rules one change to the golden file must keep. [`check`]
//! compares a commit's file with its parent's and returns every fault; `engine-cli guard`
//! applies it to each commit of a pull-request range.
//!
//! The rules:
//! 1. The file is not deleted.
//! 2. A new file holds exactly one ledger entry, a `bootstrap`, and exactly one hash set,
//!    keyed by that entry's machine.
//! 3. The old ledger entries stay, unchanged and in order; one commit adds at most one entry,
//!    and never a `bootstrap`.
//! 4. A change to the fixture list, the state inventory version, or the checkpoint spacing
//!    needs a gate-schema increase and a new `regenerate` entry; the gate schema never
//!    decreases.
//! 5. A change to the gate schema, the toolchain, or a hash set in both files, or a removed
//!    hash set, needs a new `regenerate` entry.
//! 6. A new `add-machine-set` entry adds exactly one hash set, keyed by its machine, and
//!    changes nothing else.
//! 7. A new hash set needs a new entry, and a new entry needs a change it records. The file
//!    also keeps the ledger rules of the strict load ([`golden::check_ledger`]).

use std::fmt;

use super::golden::{self, EntryKind, GoldenFile, LedgerEntry};

/// One broken rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fault {
    /// The rule number, 1 to 7; 0 when a file cannot be read.
    pub rule: u8,
    pub message: String,
}

impl fmt::Display for Fault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "rule {}: {}", self.rule, self.message)
    }
}

fn fault(rule: u8, message: impl Into<String>) -> Fault {
    Fault {
        rule,
        message: message.into(),
    }
}

/// The entries `child` adds to `parent`'s ledger, with their indices; all of `child`'s when
/// there is no parent. Empty when the child does not extend the parent's ledger.
pub fn new_entries<'a>(
    parent: Option<&GoldenFile>,
    child: &'a GoldenFile,
) -> Vec<(usize, &'a LedgerEntry)> {
    let old = parent.map_or(0, |p| p.ledger.len());
    child.ledger.iter().enumerate().skip(old).collect()
}

/// Every fault of the change from `parent` to `child` (the file texts; `None` when the file
/// is absent).
pub fn check(parent: Option<&str>, child: Option<&str>) -> Vec<Fault> {
    let Some(child) = child else {
        return vec![fault(1, "the golden file is deleted")];
    };
    let child = match golden::read(child) {
        Ok(file) => file,
        Err(e) => return vec![fault(0, format!("the new file cannot be read: {e}"))],
    };
    let mut faults = Vec::new();
    if let Err(e) = golden::check_ledger(&child) {
        faults.push(fault(7, e.to_string()));
    }
    let Some(parent) = parent else {
        let single = child.ledger.len() == 1
            && child.ledger[0].kind == EntryKind::Bootstrap
            && child.hash_sets.len() == 1
            && child.hash_sets.contains_key(&child.ledger[0].machine);
        if !single {
            faults.push(fault(
                2,
                format!(
                    "a new golden file must hold one bootstrap entry and that machine's hash \
                     set; it holds {} entries and {} hash sets",
                    child.ledger.len(),
                    child.hash_sets.len()
                ),
            ));
        }
        return faults;
    };
    let parent = match golden::read(parent) {
        Ok(file) => file,
        Err(e) => {
            faults.push(fault(0, format!("the old file cannot be read: {e}")));
            return faults;
        }
    };
    pair(&parent, &child, &mut faults);
    faults
}

fn pair(parent: &GoldenFile, child: &GoldenFile, faults: &mut Vec<Fault>) {
    // Rule 3: the old entries stay.
    for (i, old) in parent.ledger.iter().enumerate() {
        match child.ledger.get(i) {
            Some(new) if new == old => {}
            Some(_) => faults.push(fault(3, format!("ledger entry {i} is edited"))),
            None => faults.push(fault(3, format!("ledger entry {i} is removed"))),
        }
    }
    let added = new_entries(Some(parent), child);
    if added.len() > 1 {
        faults.push(fault(
            3,
            format!(
                "{} ledger entries are added; one change adds at most one",
                added.len()
            ),
        ));
    }
    for (i, e) in &added {
        if e.kind == EntryKind::Bootstrap {
            faults.push(fault(3, format!("ledger entry {i} is a second bootstrap")));
        }
    }
    let regenerate = added.iter().any(|(_, e)| e.kind == EntryKind::Regenerate);
    let add_set: Vec<&LedgerEntry> = added
        .iter()
        .filter(|(_, e)| e.kind == EntryKind::AddMachineSet)
        .map(|(_, e)| *e)
        .collect();

    // Rule 4: a fixture, inventory, or spacing change needs a schema increase.
    let mut shape = Vec::new();
    if parent.fixtures != child.fixtures {
        shape.push("the fixture list");
    }
    if parent.inventory_version != child.inventory_version {
        shape.push("the state inventory version");
    }
    if parent.checkpoint_every != child.checkpoint_every {
        shape.push("the checkpoint spacing");
    }
    if child.gate_schema < parent.gate_schema {
        faults.push(fault(
            4,
            format!(
                "the gate schema decreases from {} to {}",
                parent.gate_schema, child.gate_schema
            ),
        ));
    }
    if !shape.is_empty() {
        let what = shape.join(", ");
        if child.gate_schema <= parent.gate_schema {
            faults.push(fault(
                4,
                format!("{what} changes, but the gate schema is not increased"),
            ));
        }
        if !regenerate {
            faults.push(fault(
                4,
                format!("{what} changes with no new regenerate entry"),
            ));
        }
    }

    // Rule 5: a hash or toolchain change needs a regenerate entry.
    let mut changes = Vec::new();
    if parent.gate_schema != child.gate_schema {
        changes.push("the gate schema changes".to_string());
    }
    if parent.toolchain != child.toolchain {
        changes.push(format!(
            "the toolchain changes from {} to {}",
            parent.toolchain, child.toolchain
        ));
    }
    for (machine, set) in &parent.hash_sets {
        match child.hash_sets.get(machine) {
            Some(new) if new == set => {}
            Some(_) => changes.push(format!("hash set {machine} changes")),
            None => changes.push(format!("hash set {machine} is removed")),
        }
    }
    if !regenerate {
        for what in &changes {
            faults.push(fault(5, format!("{what} with no new regenerate entry")));
        }
    }

    // Rule 6: an add-machine-set entry adds its one set and changes nothing else.
    let new_sets: Vec<&String> = child
        .hash_sets
        .keys()
        .filter(|m| !parent.hash_sets.contains_key(*m))
        .collect();
    for e in &add_set {
        if new_sets.len() != 1 || new_sets[0] != &e.machine {
            faults.push(fault(
                6,
                format!(
                    "the add-machine-set entry for {} must add exactly that hash set; the \
                     new sets are: {}",
                    e.machine,
                    if new_sets.is_empty() {
                        "none".to_string()
                    } else {
                        new_sets
                            .iter()
                            .map(|s| s.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    }
                ),
            ));
        }
        for what in shape
            .iter()
            .map(|s| format!("{s} changes"))
            .chain(changes.clone())
        {
            faults.push(fault(
                6,
                format!("the add-machine-set entry for {} also: {what}", e.machine),
            ));
        }
    }

    // Rule 7: a new set needs an entry, and an entry needs a change.
    if added.is_empty() && !new_sets.is_empty() {
        for m in &new_sets {
            faults.push(fault(7, format!("hash set {m} is added with no new entry")));
        }
    }
    if regenerate && shape.is_empty() && changes.is_empty() && new_sets.is_empty() {
        faults.push(fault(
            7,
            "a regenerate entry is added, but no hash, header, or fixture changes",
        ));
    }
}

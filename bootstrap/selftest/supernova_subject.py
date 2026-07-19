"""Tracked typed source templates for the F5 mini-supernova rehearsal subject.

R5: the rehearsal subject is NOT a crates/ tree and is never written into the
cleanroom. These template strings ARE the tracked law; supernova.py
materializes them into the external campaign root at run time (the tier0.py
fabrication idiom). The subject is a four-crate miniature realizing a frozen
BatPak law in small: a deterministic, append-only event ledger with typed
refusals and replay (mini-ledger), lost-ack reconciliation through the
append-only boundary (mini-reconcile), bounded dependency-first progress
(mini-frontier), and the boundary executable the independent witness attacks
(mini-witness-target).

Also tracked here: the four evaluation-vector sets (search and holdout
DISJOINT by law), the campaign policy (declared search budget, fuzz seed and
bounds), the deliberately BROKEN first mini-reconcile candidate (a genuine
type error the compiler refuses), the mechanical repair-patch enumeration the
bounded search walks, the planted semantic mutant (compiles, passes the
deliberately insufficient happy-path test, changes semantics), the compiling
scaffold that must stay unrealized, and the law-changing candidate content
that stops the campaign as ArchitectRequired.
"""
from __future__ import annotations

# ---------------------------------------------------------------------------
# mini-ledger: the executable semantic relation in miniature. The happy-path
# unit test is DELIBERATELY insufficient (no overdraft case): that is what
# lets the planted mutant pass it while changing semantics.
# ---------------------------------------------------------------------------
MINI_LEDGER_RS = '''\
//! mini-ledger: a frozen BatPak law in miniature -- deterministic transition,
//! append-only evidence, typed refusal at the boundary, and replay.

/// One ledger event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// Credit the balance.
    Credit(u64),
    /// Debit the balance; refused when it would overdraw.
    Debit(u64),
}

/// The typed boundary refusal. A refusal is a lawful answer, never a crash.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The debit exceeds the balance; nothing was appended.
    InsufficientBalance { balance: u64, debit: u64 },
}

/// The append-only ledger: events are appended, never edited or removed.
#[derive(Default)]
pub struct Ledger {
    events: Vec<Event>,
    balance: u64,
}

impl Ledger {
    pub fn new() -> Ledger {
        Ledger { events: Vec::new(), balance: 0 }
    }

    pub fn balance(&self) -> u64 {
        self.balance
    }

    pub fn events(&self) -> &[Event] {
        self.events.as_slice()
    }

    /// The deterministic transition. A refused event appends nothing and
    /// edits nothing; an accepted event is appended after the ledger commits
    /// to accepting it.
    pub fn append(&mut self, event: Event) -> Result<u64, Refusal> {
        match event {
            Event::Credit(v) => {
                self.balance += v;
                self.events.push(event);
                Ok(self.balance)
            }
            Event::Debit(v) => {
                if v > self.balance {
                    return Err(Refusal::InsufficientBalance {
                        balance: self.balance,
                        debit: v,
                    });
                }
                self.balance -= v;
                self.events.push(event);
                Ok(self.balance)
            }
        }
    }

    /// Replay a captured event sequence from genesis.
    pub fn replay(events: &[Event]) -> Result<u64, Refusal> {
        let mut ledger = Ledger::new();
        let mut last = 0;
        for event in events {
            last = ledger.append(*event)?;
        }
        Ok(last)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // DELIBERATELY INSUFFICIENT happy path: it never exercises an overdraft,
    // so a mutant that silently saturates instead of refusing passes it.
    #[test]
    fn happy_path_credits_then_partial_debit() {
        let mut ledger = Ledger::new();
        assert_eq!(ledger.append(Event::Credit(10)), Ok(10));
        assert_eq!(ledger.append(Event::Debit(4)), Ok(6));
        assert_eq!(ledger.balance(), 6);
        assert_eq!(Ledger::replay(ledger.events()), Ok(6));
    }
}
'''

# The planted semantic mutant: exactly the Debit refusal arm replaced by a
# silent saturation. It compiles, passes the happy-path test above, and
# changes semantics: an overdraft is absorbed instead of refused.
MUTANT_TARGET = """\
                if v > self.balance {
                    return Err(Refusal::InsufficientBalance {
                        balance: self.balance,
                        debit: v,
                    });
                }
                self.balance -= v;
"""
MUTANT_REPLACEMENT = """\
                self.balance = self.balance.saturating_sub(v);
"""

# ---------------------------------------------------------------------------
# mini-reconcile: lost-ack reconciliation through the append-only boundary.
# __APPEND_ARG__ is the typed template hole the candidate generation fills:
# the broken first candidate passes `event` (a genuine E0308 type error the
# real compiler refuses); the repair search walks REPAIR_PATCHES.
# ---------------------------------------------------------------------------
MINI_RECONCILE_RS = '''\
//! mini-reconcile: lost-ack reconciliation in miniature -- merge remote
//! events into the local ledger through the append-only boundary. The merge
//! stops at the first typed refusal; it never rewrites prior evidence.

use mini_ledger::{Event, Ledger, Refusal};

/// Merge remote events into the local ledger; returns the final balance.
pub fn reconcile(local: &mut Ledger, remote: &[Event]) -> Result<u64, Refusal> {
    let mut last = local.balance();
    for event in remote {
        last = local.append(__APPEND_ARG__)?;
    }
    Ok(last)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merges_remote_events_through_the_boundary() {
        let mut local = Ledger::new();
        assert_eq!(local.append(Event::Credit(5)), Ok(5));
        assert_eq!(reconcile(&mut local, &[Event::Credit(3), Event::Debit(2)]), Ok(6));
        assert_eq!(local.events().len(), 3);
    }

    #[test]
    fn refusal_stops_the_merge_without_rewriting() {
        let mut local = Ledger::new();
        assert_eq!(local.append(Event::Credit(2)), Ok(2));
        let refused = reconcile(&mut local, &[Event::Credit(1), Event::Debit(9)]);
        assert_eq!(refused, Err(Refusal::InsufficientBalance { balance: 3, debit: 9 }));
        assert_eq!(local.events().len(), 2);
    }
}
'''

# The broken first candidate's argument (E0308: expected `Event`, found
# `&Event`) and the mechanical repair enumeration the bounded search walks in
# order. The first patch also fails to compile, so the receipted search
# genuinely rejects a candidate before it succeeds.
BROKEN_APPEND_ARG = "event"
REPAIR_PATCHES = ("&event", "*event", "event.clone()")

# ---------------------------------------------------------------------------
# mini-frontier: bounded dependency-first progress, plus the compiling
# scaffold that must stay unrealized (RealizationPosture::Scaffold).
# ---------------------------------------------------------------------------
MINI_FRONTIER_RS = '''\
//! mini-frontier: bounded progress in miniature -- advance a trusted
//! frontier over reconciliation batches, dependency-first, never past a
//! refusal, never past the declared budget.

use mini_ledger::{Event, Ledger, Refusal};
use mini_reconcile::reconcile;

/// The typed outcome of a bounded frontier advance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrontierOutcome {
    /// Every batch reconciled within budget.
    Advanced { batches: usize, balance: u64 },
    /// A batch was refused; the frontier stops AT the refusal.
    RefusedAt { batch: usize, refusal: Refusal },
    /// The declared budget ran out before the batches did.
    BudgetExhausted { batches: usize },
}

/// Advance the frontier over `batches`, at most `max_batches` of them.
pub fn advance(local: &mut Ledger, batches: &[&[Event]], max_batches: usize) -> FrontierOutcome {
    let mut done = 0;
    let mut balance = local.balance();
    for batch in batches {
        if done == max_batches {
            return FrontierOutcome::BudgetExhausted { batches: done };
        }
        match reconcile(local, batch) {
            Ok(b) => {
                balance = b;
                done += 1;
            }
            Err(refusal) => return FrontierOutcome::RefusedAt { batch: done, refusal },
        }
    }
    FrontierOutcome::Advanced { batches: done, balance }
}

/// SCAFFOLD -- must stay unrealized. A compiling placeholder for the
/// drain-and-reorder obligation: it type-checks and realizes nothing, so it
/// counts in the UNREALIZED denominator and can never close its obligation.
pub fn drain_reorder_scaffold() -> Option<u64> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advances_within_budget_and_stops_at_refusal() {
        let mut local = Ledger::new();
        let a: &[Event] = &[Event::Credit(4)];
        let b: &[Event] = &[Event::Debit(9)];
        assert_eq!(
            advance(&mut local, &[a], 2),
            FrontierOutcome::Advanced { batches: 1, balance: 4 }
        );
        assert_eq!(
            advance(&mut local, &[b], 2),
            FrontierOutcome::RefusedAt {
                batch: 0,
                refusal: Refusal::InsufficientBalance { balance: 4, debit: 9 }
            }
        );
        assert_eq!(advance(&mut local, &[a, a], 1), FrontierOutcome::BudgetExhausted { batches: 1 });
    }
}
'''

# ---------------------------------------------------------------------------
# mini-witness-target: the boundary executable the independent witness
# attacks. `run` prints the boundary transcript per op; `capture` appends the
# observed history (append-only continuation: an existing history is replayed
# to rebuild state, then new observations are APPENDED, never rewritten). The
# in-band side may state at most `no-divergence-observed` -- never a
# conformance conclusion; only the witness's offline replay concludes.
# ---------------------------------------------------------------------------
MINI_WITNESS_TARGET_RS = '''\
//! mini-witness-target: the real boundary under attack. It exposes the
//! ledger relation through the reconciliation stack, summarizes bounded
//! progress through the frontier, and never grades itself: it prints
//! observations; the independent witness judges them.

use mini_frontier::{advance, FrontierOutcome};
use mini_ledger::{Event, Ledger};
use mini_reconcile::reconcile;
use std::fs;
use std::io::Write;
use std::process::ExitCode;

fn parse_ops(text: &str) -> Result<Vec<Event>, String> {
    let mut ops = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line == "----" {
            continue;
        }
        let (kind, amount) = line.split_once(' ').ok_or_else(|| format!("bad op {line:?}"))?;
        let v: u64 = amount.parse().map_err(|_| format!("bad amount {line:?}"))?;
        match kind {
            "credit" => ops.push(Event::Credit(v)),
            "debit" => ops.push(Event::Debit(v)),
            other => return Err(format!("unknown op {other:?}")),
        }
    }
    Ok(ops)
}

fn step(local: &mut Ledger, event: Event) -> (String, bool) {
    match reconcile(local, &[event]) {
        Ok(balance) => (format!("ok {balance}"), true),
        Err(mini_ledger::Refusal::InsufficientBalance { balance, debit }) => {
            (format!("refused insufficient-balance balance={balance} debit={debit}"), false)
        }
    }
}

fn run(trace_path: &str, max_batches: usize) -> Result<(), String> {
    let text = fs::read_to_string(trace_path).map_err(|e| format!("read {trace_path}: {e}"))?;
    let ops = parse_ops(&text)?;
    let mut local = Ledger::new();
    let mut applied = 0u64;
    let mut refused = 0u64;
    for event in &ops {
        let (line, ok) = step(&mut local, *event);
        println!("{line}");
        if ok {
            applied += 1;
        } else {
            refused += 1;
        }
    }
    println!("final {} applied={applied} refused={refused}", local.balance());
    // Bounded dependency-first progress over the same ops, one batch per op,
    // on a fresh ledger: the frontier stops AT the first refusal and never
    // advances past the declared budget.
    let batches: Vec<&[Event]> = ops.chunks(1).collect();
    let mut fresh = Ledger::new();
    match advance(&mut fresh, &batches, max_batches) {
        FrontierOutcome::Advanced { batches, balance } => {
            println!("frontier advanced batches={batches} balance={balance}");
        }
        FrontierOutcome::RefusedAt { batch, refusal } => {
            let mini_ledger::Refusal::InsufficientBalance { balance, debit } = refusal;
            println!(
                "frontier refused-at batch={batch} insufficient-balance balance={balance} debit={debit}"
            );
        }
        FrontierOutcome::BudgetExhausted { batches } => {
            println!("frontier budget-exhausted batches={batches}");
        }
    }
    Ok(())
}

fn history_line(event: Event, outcome: &str) -> String {
    match event {
        Event::Credit(v) => format!("credit {v} {outcome}"),
        Event::Debit(v) => format!("debit {v} {outcome}"),
    }
}

fn capture(trace_path: &str, history_path: &str) -> Result<(), String> {
    // Append-only continuation: replay the existing history to rebuild state
    // (the in-band check), then APPEND new observations. Never rewrite.
    let mut local = Ledger::new();
    let prior = fs::read_to_string(history_path).unwrap_or_default();
    for line in prior.lines() {
        let mut parts = line.split(' ');
        let kind = parts.next().unwrap_or("");
        let v: u64 = parts.next().unwrap_or("0").parse().map_err(|_| "bad history")?;
        let outcome = parts.next().unwrap_or("");
        let event = match kind {
            "credit" => Event::Credit(v),
            "debit" => Event::Debit(v),
            other => return Err(format!("bad history op {other:?}")),
        };
        if outcome.starts_with("ok=") {
            let recorded: u64 = outcome[3..].parse().map_err(|_| "bad history balance")?;
            match local.append(event) {
                Ok(balance) if balance == recorded => {}
                _ => {
                    println!("mini-target: divergence-observed");
                    return Err("in-band replay diverged from recorded history".to_owned());
                }
            }
        }
        // A `refused` history line appended nothing; state is unchanged.
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(history_path)
        .map_err(|e| format!("open {history_path}: {e}"))?;
    let text = fs::read_to_string(trace_path).map_err(|e| format!("read {trace_path}: {e}"))?;
    for event in parse_ops(&text)? {
        let line = match reconcile(&mut local, &[event]) {
            Ok(balance) => history_line(event, &format!("ok={balance}")),
            Err(mini_ledger::Refusal::InsufficientBalance { balance, debit }) => {
                history_line(event, &format!("refused-insufficient balance={balance} debit={debit}"))
            }
        };
        writeln!(file, "{line}").map_err(|e| format!("append history: {e}"))?;
    }
    // In-band observation states AT MOST no-divergence; conformance is the
    // witness's offline replay conclusion alone.
    println!("mini-target: no-divergence-observed");
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let result = match args.get(1).map(String::as_str) {
        Some("run") if args.len() == 4 => match args[3].parse::<usize>() {
            Ok(max_batches) => run(&args[2], max_batches),
            Err(_) => Err("max-batches is not a usize".to_owned()),
        },
        Some("capture") if args.len() == 4 => capture(&args[2], &args[3]),
        _ => Err(
            "usage: mini-witness-target run <trace> <max-batches> | capture <trace> <history>"
                .to_owned(),
        ),
    };
    if let Err(message) = result {
        eprintln!("mini-target: FAIL {message}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
'''

# ---------------------------------------------------------------------------
# The search-evaluation driver: the bounded repair search evaluates each
# candidate patch by executing the search vectors through mini-reconcile and
# handing the transcript to the witness. Depends only on ledger + reconcile so
# a broken downstream cannot block the search evaluation.
# ---------------------------------------------------------------------------
SEARCH_DRIVER_RS = '''\
//! Search-evaluation driver: prints the boundary transcript of the search
//! vectors through mini-reconcile. The witness judges; this never grades.

use mini_ledger::{Event, Ledger};
use mini_reconcile::reconcile;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let Some(path) = args.get(1) else {
        eprintln!("usage: search-driver <trace-file>");
        return ExitCode::FAILURE;
    };
    let Ok(text) = fs::read_to_string(path) else {
        eprintln!("search-driver: cannot read {path}");
        return ExitCode::FAILURE;
    };
    let mut local = Ledger::new();
    let mut applied = 0u64;
    let mut refused = 0u64;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line == "----" {
            println!("final {} applied={applied} refused={refused}", local.balance());
            println!("----");
            local = Ledger::new();
            applied = 0;
            refused = 0;
            continue;
        }
        let Some((kind, amount)) = line.split_once(' ') else {
            eprintln!("search-driver: bad op {line:?}");
            return ExitCode::FAILURE;
        };
        let Ok(v) = amount.parse::<u64>() else {
            eprintln!("search-driver: bad amount {line:?}");
            return ExitCode::FAILURE;
        };
        let event = if kind == "credit" { Event::Credit(v) } else { Event::Debit(v) };
        match reconcile(&mut local, &[event]) {
            Ok(balance) => {
                println!("ok {balance}");
                applied += 1;
            }
            Err(mini_ledger::Refusal::InsufficientBalance { balance, debit }) => {
                println!("refused insufficient-balance balance={balance} debit={debit}");
                refused += 1;
            }
        }
    }
    println!("final {} applied={applied} refused={refused}", local.balance());
    ExitCode::SUCCESS
}
'''

# ---------------------------------------------------------------------------
# The law-changing candidate content. NEVER compiled, NEVER qualified: its
# admission stops the campaign as ArchitectRequired -- a candidate does not
# amend the realized law from inside the campaign.
# ---------------------------------------------------------------------------
LAW_CHANGE_PATCH = """\
// LAW-CHANGING candidate: permit overdraft up to 10 units. This amends the
// InsufficientBalance refusal law itself (docs-owned), so it cannot ride any
// realization-preserving repair lane: ArchitectRequired.
                if v > self.balance + 10 {
"""

# ---------------------------------------------------------------------------
# Evaluation vectors. Traces are op lines; "----" separates traces. Search
# and holdout are DISJOINT by law (the harness refuses an overlap before any
# evaluation runs). The qualification vectors carry the overdraft cases that
# kill the mutant.
# ---------------------------------------------------------------------------
SEARCH_VECTORS = """\
credit 7
debit 3
debit 5
----
credit 2
credit 3
debit 5
----
debit 1
credit 4
"""

QUALIFICATION_VECTORS = """\
credit 10
debit 4
debit 7
debit 6
----
credit 1
debit 1
debit 1
----
credit 20
debit 19
debit 2
credit 1
debit 2
"""

HOLDOUT_VECTORS = """\
credit 8
debit 9
debit 8
----
credit 3
credit 4
debit 6
debit 2
"""

REGRESSION_VECTORS = """\
credit 5
debit 5
----
credit 0
debit 0
"""

CAPTURE_TRACE_1 = """\
credit 6
debit 2
"""

CAPTURE_TRACE_2 = """\
credit 3
debit 8
debit 7
"""

# ---------------------------------------------------------------------------
# The frozen campaign policy: the declared search budget (receipted against
# actual use), the seeded fuzz bounds (R2: deterministic PRNG; candidate and
# confirming runs reproduce the identical attack), and the loop bounds.
# ---------------------------------------------------------------------------
POLICY_TEXT = """\
BATPAK-CAMPAIGN-POLICY/1
search-budget max-candidates=4 max-logical-work=100000 max-memory-bytes=1048576 max-monotonic-ticks=120000000000
fuzz seed=3735928559 traces=48 max-ops=12 max-amount=9
frontier max-batches=8
repair max-loop=3
"""

FUZZ_SEED = 3735928559
FUZZ_TRACES = 48
FUZZ_MAX_OPS = 12
FUZZ_MAX_AMOUNT = 9

SEARCH_BUDGET = {
    "max-candidates": 4,
    "max-logical-work": 100000,
    "max-memory-bytes": 1048576,
    "max-monotonic-ticks": 120000000000,
}

REPAIR_MAX_LOOP = 3
FRONTIER_MAX_BATCHES = 8

# The scaffold's exact compiled content (must appear verbatim in the frontier
# source so "the scaffold compiles" is a fact, not an assertion).
SCAFFOLD_OBLIGATION = "mini-frontier/drain-reorder"
SCAFFOLD_CONTENT = """\
pub fn drain_reorder_scaffold() -> Option<u64> {
    None
}
"""

# Unit names, dependency edges (importer -> importee), and the vector roles.
UNITS = ("mini-ledger", "mini-reconcile", "mini-frontier", "mini-witness-target")
DEPENDENCY_EDGES = (
    ("mini-reconcile", "mini-ledger"),
    ("mini-frontier", "mini-reconcile"),
    ("mini-witness-target", "mini-frontier"),
)
VECTOR_SETS = {
    "search": SEARCH_VECTORS,
    "qualification": QUALIFICATION_VECTORS,
    "holdout": HOLDOUT_VECTORS,
    "regression": REGRESSION_VECTORS,
}

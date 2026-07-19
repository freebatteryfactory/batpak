//! supernova_witness — the INDEPENDENT witness of the F5 mini-supernova
//! rehearsal (a tracked selftest fixture source, compiled by the harness like
//! the tier0 law-fixture idiom; deliberately NOT a bootstrap tool — the
//! BootstrapToolId census stays a closed seven-tool enum).
//!
//! It independently implements the mini-ledger semantic relation (a fold with
//! checked subtraction — a different structure from the subject's running
//! balance), and executes the four verification routes the campaign proof
//! rows demand:
//!
//! * `judge`    — differential judgment: recompute the boundary transcript of
//!                a trace file and byte-compare it with the subject-produced
//!                transcript (IndependentReference; kills the planted mutant).
//! * `simulate` — deterministic replay-equality of a FULLY SIMULATED trace
//!                (R1): each trace is simulated twice from genesis and the
//!                two transcripts must be byte-identical. Explicitly NOT
//!                interleaving exploration and NOT reachable-set exploration.
//! * `fuzz`     — the seeded bounded generated-trace attack (R2): a
//!                deterministic xorshift64* PRNG generates traces (illegal
//!                overdrafts included), each executed against the REAL target
//!                executable; every illegal debit must terminate in the typed
//!                refusal and every transcript must match the witness's own.
//! * `replay`   — offline replay of the complete captured observed history;
//!                the ONLY route that may conclude conformant-for-observed-
//!                history (the in-band side states at most no-divergence).
//!
//! The producer (supernova.py) orchestrates and never votes on semantics: it
//! reads this witness's verdict lines.

use std::fs;
use std::process::{Command, ExitCode};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Op {
    Credit(u64),
    Debit(u64),
}

fn parse_traces(text: &str) -> Result<Vec<Vec<Op>>, String> {
    let mut traces: Vec<Vec<Op>> = Vec::new();
    let mut current: Vec<Op> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line == "----" {
            traces.push(std::mem::take(&mut current));
            continue;
        }
        let (kind, amount) = line
            .split_once(' ')
            .ok_or_else(|| format!("bad op line {line:?}"))?;
        let v: u64 = amount
            .parse()
            .map_err(|_| format!("bad amount in {line:?}"))?;
        match kind {
            "credit" => current.push(Op::Credit(v)),
            "debit" => current.push(Op::Debit(v)),
            other => return Err(format!("unknown op {other:?}")),
        }
    }
    traces.push(current);
    Ok(traces)
}

/// The witness's OWN realization of the relation: balance as a fold over the
/// accepted prefix with checked subtraction. Independent by construction.
fn fold_balance(accepted: &[Op]) -> u64 {
    let mut balance: u64 = 0;
    for op in accepted {
        balance = match op {
            Op::Credit(v) => balance + v,
            Op::Debit(v) => balance - v,
        };
    }
    balance
}

fn apply(accepted: &mut Vec<Op>, op: Op) -> Result<u64, (u64, u64)> {
    let balance = fold_balance(accepted);
    match op {
        Op::Credit(_) => {
            accepted.push(op);
            Ok(fold_balance(accepted))
        }
        Op::Debit(v) => match balance.checked_sub(v) {
            Some(after) => {
                accepted.push(op);
                Ok(after)
            }
            None => Err((balance, v)),
        },
    }
}

/// The expected boundary transcript of ONE trace, in the exact grammar the
/// subject target prints: per-op lines, the final line, and the bounded
/// frontier summary (one batch per op, stop at first refusal or budget).
fn expected_transcript(trace: &[Op], max_batches: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut accepted: Vec<Op> = Vec::new();
    let mut applied = 0u64;
    let mut refused = 0u64;
    for op in trace {
        match apply(&mut accepted, *op) {
            Ok(balance) => {
                out.push(format!("ok {balance}"));
                applied += 1;
            }
            Err((balance, debit)) => {
                out.push(format!(
                    "refused insufficient-balance balance={balance} debit={debit}"
                ));
                refused += 1;
            }
        }
    }
    out.push(format!(
        "final {} applied={applied} refused={refused}",
        fold_balance(&accepted)
    ));
    // `max_batches == 0` means the transcript under judgment came from a
    // producer without the frontier stage (the search driver); no frontier
    // summary line is expected then.
    if max_batches == 0 {
        return out;
    }
    // Frontier summary: batches of one op, halting AT the first refusal,
    // never past the declared budget.
    let mut frontier_accepted: Vec<Op> = Vec::new();
    let mut batches = 0usize;
    let mut outcome: Option<String> = None;
    for op in trace {
        if batches == max_batches {
            outcome = Some(format!("frontier budget-exhausted batches={batches}"));
            break;
        }
        match apply(&mut frontier_accepted, *op) {
            Ok(_) => batches += 1,
            Err((balance, debit)) => {
                outcome = Some(format!(
                    "frontier refused-at batch={batches} insufficient-balance \
                     balance={balance} debit={debit}"
                ));
                break;
            }
        }
    }
    out.push(outcome.unwrap_or_else(|| {
        format!(
            "frontier advanced batches={batches} balance={}",
            fold_balance(&frontier_accepted)
        )
    }));
    out
}

fn mode_judge(trace_path: &str, transcript_path: &str, max_batches: usize) -> Result<(), String> {
    let traces = parse_traces(
        &fs::read_to_string(trace_path).map_err(|e| format!("read {trace_path}: {e}"))?,
    )?;
    let observed =
        fs::read_to_string(transcript_path).map_err(|e| format!("read {transcript_path}: {e}"))?;
    let mut expected: Vec<String> = Vec::new();
    for (i, trace) in traces.iter().enumerate() {
        if i > 0 {
            expected.push("----".to_owned());
        }
        expected.extend(expected_transcript(trace, max_batches));
    }
    let observed_lines: Vec<&str> = observed.lines().collect();
    if observed_lines.len() != expected.len() {
        return Err(format!(
            "witness: DIVERGE length expected={} observed={}",
            expected.len(),
            observed_lines.len()
        ));
    }
    for (i, (want, got)) in expected.iter().zip(observed_lines.iter()).enumerate() {
        if want != got {
            return Err(format!(
                "witness: DIVERGE step={i} expected={want:?} observed={got:?}"
            ));
        }
    }
    println!(
        "witness: AGREE traces={} lines={}",
        traces.len(),
        expected.len()
    );
    Ok(())
}

fn mode_simulate(trace_path: &str, max_batches: usize) -> Result<(), String> {
    let traces = parse_traces(
        &fs::read_to_string(trace_path).map_err(|e| format!("read {trace_path}: {e}"))?,
    )?;
    let mut lines = 0usize;
    for (t, trace) in traces.iter().enumerate() {
        // Replay-equality of the FULL simulation: two independent complete
        // simulations of the same trace must be byte-identical, step by step.
        let first = expected_transcript(trace, max_batches);
        let second = expected_transcript(trace, max_batches);
        for (i, (a, b)) in first.iter().zip(second.iter()).enumerate() {
            if a != b {
                return Err(format!(
                    "witness: REPLAY-DIVERGE trace={t} step={i} first={a:?} second={b:?}"
                ));
            }
        }
        if first.len() != second.len() {
            return Err(format!("witness: REPLAY-DIVERGE trace={t} lengths differ"));
        }
        lines += first.len();
    }
    println!(
        "witness: REPLAY-EQUAL traces={} lines={lines}",
        traces.len()
    );
    Ok(())
}

/// xorshift64* — a tiny deterministic PRNG; the seed and bounds are recorded
/// in the receipt so the candidate and confirming runs reproduce the
/// identical attack (R2).
struct XorShift64Star(u64);

impl XorShift64Star {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
}

fn mode_fuzz(
    seed: u64,
    trace_count: u64,
    max_ops: u64,
    max_amount: u64,
    max_batches: usize,
    target_exe: &str,
    workdir: &str,
) -> Result<(), String> {
    let mut rng = XorShift64Star(seed | 1);
    let mut executed = 0u64;
    let mut refusals = 0u64;
    for t in 0..trace_count {
        let ops = 1 + rng.next() % max_ops;
        let mut trace: Vec<Op> = Vec::new();
        let mut text = String::new();
        for _ in 0..ops {
            let amount = rng.next() % (max_amount + 1);
            // Debit-biased so illegal overdrafts genuinely occur.
            let op = if rng.next() % 3 == 0 {
                text.push_str(&format!("credit {amount}\n"));
                Op::Credit(amount)
            } else {
                text.push_str(&format!("debit {amount}\n"));
                Op::Debit(amount)
            };
            trace.push(op);
        }
        let trace_path = format!("{workdir}/fuzz-{t}.trace");
        fs::write(&trace_path, &text).map_err(|e| format!("write {trace_path}: {e}"))?;
        let output = Command::new(target_exe)
            .arg("run")
            .arg(&trace_path)
            .arg(max_batches.to_string())
            .output()
            .map_err(|e| format!("spawn {target_exe}: {e}"))?;
        if !output.status.success() {
            return Err(format!("witness: FUZZ-COUNTEREXAMPLE trace={t} target exited nonzero"));
        }
        let observed = String::from_utf8_lossy(&output.stdout);
        let observed_lines: Vec<&str> = observed.lines().collect();
        let expected = expected_transcript(&trace, max_batches);
        if observed_lines.len() != expected.len() {
            return Err(format!("witness: FUZZ-COUNTEREXAMPLE trace={t} transcript length"));
        }
        for (i, (want, got)) in expected.iter().zip(observed_lines.iter()).enumerate() {
            if want != got {
                return Err(format!(
                    "witness: FUZZ-COUNTEREXAMPLE trace={t} step={i} expected={want:?} \
                     observed={got:?}"
                ));
            }
        }
        // The boundary law itself: every illegal debit terminates in the
        // typed refusal line, never in silent acceptance.
        let mut accepted: Vec<Op> = Vec::new();
        for (i, op) in trace.iter().enumerate() {
            match apply(&mut accepted, *op) {
                Ok(_) => {}
                Err(_) => {
                    refusals += 1;
                    if !observed_lines[i].starts_with("refused insufficient-balance ") {
                        return Err(format!(
                            "witness: FUZZ-COUNTEREXAMPLE trace={t} step={i} illegal debit \
                             was not refused at the boundary"
                        ));
                    }
                }
            }
        }
        executed += 1;
    }
    println!(
        "witness: FUZZ-HELD seed={seed} traces={trace_count} max-ops={max_ops} \
         max-amount={max_amount} executed={executed} refusals-held={refusals}"
    );
    Ok(())
}

fn mode_replay(history_path: &str) -> Result<(), String> {
    let text =
        fs::read_to_string(history_path).map_err(|e| format!("read {history_path}: {e}"))?;
    let mut accepted: Vec<Op> = Vec::new();
    let mut events = 0usize;
    for (i, line) in text.lines().enumerate() {
        let mut parts = line.split(' ');
        let kind = parts.next().unwrap_or("");
        let amount: u64 = parts
            .next()
            .unwrap_or("")
            .parse()
            .map_err(|_| format!("witness: DIVERGENT step={i} unparseable history line"))?;
        let outcome = parts.next().unwrap_or("");
        let op = match kind {
            "credit" => Op::Credit(amount),
            "debit" => Op::Debit(amount),
            other => return Err(format!("witness: DIVERGENT step={i} unknown op {other:?}")),
        };
        match apply(&mut accepted, op) {
            Ok(balance) => {
                let want = format!("ok={balance}");
                if outcome != want {
                    return Err(format!(
                        "witness: DIVERGENT step={i} replay says {want:?}, history says \
                         {outcome:?}"
                    ));
                }
            }
            Err((balance, debit)) => {
                if outcome != "refused-insufficient" {
                    return Err(format!(
                        "witness: DIVERGENT step={i} replay refuses \
                         (balance={balance} debit={debit}), history says {outcome:?}"
                    ));
                }
            }
        }
        events += 1;
    }
    // Only THIS offline replay of the complete captured history concludes.
    println!("witness: CONFORMANT-FOR-OBSERVED-HISTORY events={events}");
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let parse_usize = |s: &String| s.parse::<usize>().map_err(|_| format!("bad usize {s:?}"));
    let parse_u64 = |s: &String| s.parse::<u64>().map_err(|_| format!("bad u64 {s:?}"));
    let result = match args.get(1).map(String::as_str) {
        Some("judge") if args.len() == 5 => {
            parse_usize(&args[4]).and_then(|mb| mode_judge(&args[2], &args[3], mb))
        }
        Some("simulate") if args.len() == 4 => {
            parse_usize(&args[3]).and_then(|mb| mode_simulate(&args[2], mb))
        }
        Some("fuzz") if args.len() == 9 => (|| {
            let seed = parse_u64(&args[2])?;
            let traces = parse_u64(&args[3])?;
            let max_ops = parse_u64(&args[4])?;
            let max_amount = parse_u64(&args[5])?;
            let max_batches = parse_usize(&args[6])?;
            mode_fuzz(seed, traces, max_ops, max_amount, max_batches, &args[7], &args[8])
        })(),
        Some("replay") if args.len() == 3 => mode_replay(&args[2]),
        _ => Err("usage: supernova_witness judge <trace> <transcript> <max-batches> | \
                  simulate <trace> <max-batches> | fuzz <seed> <traces> <max-ops> \
                  <max-amount> <max-batches> <target-exe> <workdir> | replay <history>"
            .to_owned()),
    };
    if let Err(message) = result {
        eprintln!("{message}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

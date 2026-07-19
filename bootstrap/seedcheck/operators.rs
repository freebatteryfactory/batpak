use spec::{architecture, dispositions, guarantees, legacy_obligations, operators, proof, verification};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use crate::proof::contract_authored_texts;

/// The typed operator surface law, EXECUTED (5.5E3j). Seedcheck runs the real
/// Rust values: identity/row parity in `OperatorId::ALL` order, token
/// discipline on both closed surface inventories, owner and basis resolution,
/// surface adoption, and the class/shape law. It parses no Markdown and no
/// decision prose; those reconstructions belong to audit.py.
pub(crate) fn check_operators(root: &Path, findings: &mut Vec<String>) {
    let owner_texts = contract_authored_texts(root);
    let decision_ids: BTreeSet<&str> = dispositions::DECISIONS.iter().map(|d| d.id).collect();
    let mut spellings: BTreeSet<&str> = BTreeSet::new();
    for id in operators::OperatorId::ALL {
        let spelling = id.spelling();
        if spelling.trim().is_empty() {
            findings.push("empty OperatorId spelling".into());
        }
        if !spellings.insert(spelling) {
            findings.push(format!("duplicate OperatorId spelling {spelling}"));
        }
        let rows = operators::OPERATORS.iter().filter(|o| o.id == *id).count();
        if rows != 1 {
            findings.push(format!(
                "OperatorId {spelling} has {rows} OperatorSpec rows; exactly one is lawful"));
        }
        if !owner_texts.contains_key(id.semantic_owner().raw()) {
            findings.push(format!(
                "operator {spelling} names owner {}, which no document declares",
                id.semantic_owner().raw()));
        }
        if !decision_ids.contains(id.admission_basis().raw()) {
            findings.push(format!(
                "operator {spelling} names admission basis {}, which no declared decision owns",
                id.admission_basis().raw()));
        }
    }
    if operators::OPERATORS.len() != operators::OperatorId::ALL.len() {
        findings.push(format!(
            "OPERATORS declares {} rows for {} OperatorId variants",
            operators::OPERATORS.len(),
            operators::OperatorId::ALL.len()));
    }
    for (index, op) in operators::OPERATORS.iter().enumerate() {
        if operators::OperatorId::ALL.get(index) != Some(&op.id) {
            findings.push(format!(
                "OPERATORS row {} is out of OperatorId::ALL order", op.id.spelling()));
        }
    }
    // The closed word inventory: canonical uppercase word grammar, unique
    // nonempty tokens, resolved owner and basis, exactly one adopting row.
    let mut word_tokens: BTreeSet<&str> = BTreeSet::new();
    for word in operators::OperatorWordSurface::ALL {
        let token = word.token();
        let uppercase_words = !token.is_empty()
            && !token.starts_with(' ')
            && !token.ends_with(' ')
            && !token.contains("  ")
            && token.bytes().all(|b| b.is_ascii_uppercase() || b == b' ');
        if !uppercase_words {
            findings.push(format!(
                "word surface token {token:?} violates the canonical uppercase word grammar"));
        }
        if !word_tokens.insert(token) {
            findings.push(format!("duplicate word surface token {token:?}"));
        }
        if !owner_texts.contains_key(word.semantic_owner().raw()) {
            findings.push(format!(
                "word surface {token:?} names owner {}, which no document declares",
                word.semantic_owner().raw()));
        }
        if !decision_ids.contains(word.admission_basis().raw()) {
            findings.push(format!(
                "word surface {token:?} names admission basis {}, which no declared decision owns",
                word.admission_basis().raw()));
        }
        let adopters = operators::OPERATORS
            .iter()
            .filter(|o| o.syntax.canonical_word() == Some(*word))
            .count();
        if adopters != 1 {
            findings.push(format!(
                "word surface {token:?} is adopted by {adopters} OperatorSpec rows; exactly one is lawful"));
        }
    }
    // The closed symbol inventory: nonempty ASCII punctuation, unique tokens,
    // resolved owner and basis, exactly one adopting row — an alias stays
    // attached to its one canonical OperatorId.
    let mut symbol_tokens: BTreeSet<&str> = BTreeSet::new();
    for symbol in operators::OperatorSymbolSurface::ALL {
        let token = symbol.token();
        if token.is_empty() || !token.bytes().all(|b| b.is_ascii_punctuation()) {
            findings.push(format!(
                "symbol surface token {token:?} is not nonempty ASCII punctuation"));
        }
        if !symbol_tokens.insert(token) {
            findings.push(format!("duplicate symbol surface token {token:?}"));
        }
        if !owner_texts.contains_key(symbol.semantic_owner().raw()) {
            findings.push(format!(
                "symbol surface {token:?} names owner {}, which no document declares",
                symbol.semantic_owner().raw()));
        }
        if !decision_ids.contains(symbol.admission_basis().raw()) {
            findings.push(format!(
                "symbol surface {token:?} names admission basis {}, which no declared decision owns",
                symbol.admission_basis().raw()));
        }
        let adopters = operators::OPERATORS
            .iter()
            .filter(|o| {
                o.syntax.canonical_symbol() == Some(*symbol)
                    || o.syntax.symbol_alias() == Some(*symbol)
            })
            .count();
        if adopters != 1 {
            findings.push(format!(
                "symbol surface {token:?} is adopted by {adopters} OperatorSpec rows; exactly one is lawful"));
        }
    }
    let mut surface_owner: BTreeMap<(&str, &str), &str> = BTreeMap::new();
    for op in operators::OPERATORS {
        let spelling = op.id.spelling();
        if op.syntax.canonical_token().is_empty() {
            findings.push(format!("operator {spelling} has an empty canonical token"));
        }
        if op.semantic_op.trim().is_empty()
            || op.input_sorts.trim().is_empty()
            || op.result_sort.trim().is_empty()
            || op.overflow.trim().is_empty()
            || op.exception.trim().is_empty()
            || op.spoken.trim().is_empty()
            || op.mutation_classes.trim().is_empty()
        {
            findings.push(format!("incomplete operator {spelling}"));
        }
        // The class/shape law is total: arithmetic is symbol-only, comparison
        // is word-with-symbol-alias, logical is word-only. Exhaustive on both
        // axes so a new class or shape must be classified here, not defaulted.
        let lawful_shape = match (op.class, op.syntax) {
            (operators::OperatorClass::Arithmetic, operators::OperatorSyntax::SymbolOnly(_)) => true,
            (operators::OperatorClass::Arithmetic, operators::OperatorSyntax::WordOnly(_))
            | (operators::OperatorClass::Arithmetic, operators::OperatorSyntax::WordWithSymbolAlias(_, _)) => false,
            (operators::OperatorClass::Comparison, operators::OperatorSyntax::WordWithSymbolAlias(_, _)) => true,
            (operators::OperatorClass::Comparison, operators::OperatorSyntax::SymbolOnly(_))
            | (operators::OperatorClass::Comparison, operators::OperatorSyntax::WordOnly(_)) => false,
            (operators::OperatorClass::Logical, operators::OperatorSyntax::WordOnly(_)) => true,
            (operators::OperatorClass::Logical, operators::OperatorSyntax::SymbolOnly(_))
            | (operators::OperatorClass::Logical, operators::OperatorSyntax::WordWithSymbolAlias(_, _)) => false,
        };
        if !lawful_shape {
            findings.push(format!(
                "operator {spelling} violates the class/shape law: arithmetic is symbol-only, \
                 comparison is word-with-symbol-alias, logical is word-only"));
        }
        let fixity = match op.fixity {
            operators::Fixity::Prefix => "Prefix",
            operators::Fixity::Infix => "Infix",
        };
        let alias_token = match op.syntax.symbol_alias() {
            Some(symbol) => symbol.token(),
            None => "",
        };
        for surface in [op.syntax.canonical_token(), alias_token] {
            if surface.is_empty() {
                continue;
            }
            if let Some(prev) = surface_owner.insert((surface, fixity), spelling) {
                if prev != spelling {
                    findings.push(format!(
                        "operator token {surface} fixity {fixity} claimed by {prev} and {spelling}"));
                }
            }
        }
        // Typed legality rules (5.5E1): placement is law. A wall-observation
        // difference anywhere but subtraction would leak wall arithmetic past
        // the TimeDelta fence (docs/16); a rate difference IS subtraction.
        if op.typing.is_empty() {
            findings.push(format!("operator {spelling} declares no typing rules"));
        }
        for rule in op.typing {
            // Exhaustive: a new rule must be classified here, not defaulted.
            match rule {
                operators::OperatorTypingRule::WallObservationDifference
                | operators::OperatorTypingRule::PercentDifference => {
                    if op.semantic_op != "subtract" {
                        findings.push(format!(
                            "operator {spelling} claims a difference typing rule but is not subtraction"));
                    }
                }
                operators::OperatorTypingRule::PercentAdjustment => {
                    if op.semantic_op != "add" && op.semantic_op != "subtract" {
                        findings.push(format!(
                            "operator {spelling} claims PercentAdjustment outside add/subtract"));
                    }
                }
                operators::OperatorTypingRule::SameSortComparison => {
                    if !matches!(op.class, operators::OperatorClass::Comparison) {
                        findings.push(format!(
                            "operator {spelling} claims SameSortComparison outside the comparison class"));
                    }
                }
                operators::OperatorTypingRule::TruthUnary
                | operators::OperatorTypingRule::TruthBinary => {
                    if !matches!(op.class, operators::OperatorClass::Logical) {
                        findings.push(format!(
                            "operator {spelling} claims a Truth typing rule outside the logical class"));
                    }
                }
                operators::OperatorTypingRule::SameUnit
                | operators::OperatorTypingRule::DimensionalByDimensionless
                | operators::OperatorTypingRule::LikeDimensionRatio => {
                    if !matches!(op.class, operators::OperatorClass::Arithmetic) {
                        findings.push(format!(
                            "operator {spelling} claims an arithmetic typing rule outside the arithmetic class"));
                    }
                }
            }
        }
        match op.arity {
            operators::Arity::Unary | operators::Arity::Binary => {}
        }
        match op.associativity {
            operators::Associativity::Left
            | operators::Associativity::Right
            | operators::Associativity::NonAssociative => {}
        }
        match op.exactness {
            operators::Exactness::Exact | operators::Exactness::NotApplicable => {}
        }
        match op.numeric_support {
            operators::NumericSupport::ExactSupported
            | operators::NumericSupport::QualifiedProfileOnly
            | operators::NumericSupport::Unsupported
            | operators::NumericSupport::NotApplicable => {}
        }
    }
}

/// The typed proof relations and complete legacy witness routes, EXECUTED
/// (5.5E4d). Seedcheck runs the real values: nonempty supplemental fields,
/// exactly one witness route per obligation with the matching relation
/// posture, resolvable currently-active guarantees, nonempty duplicate-free
/// resolvable projection contracts, active retired-successors, and the
/// LEG/DEC-only guarantee family law. It parses no docs/24 meaning prose and
/// no generated Markdown.
pub(crate) fn check_proof_relations(root: &Path, findings: &mut Vec<String>) {
    use proof::{ProofRowState, PROOF_ROWS};
    let owner_texts = contract_authored_texts(root);
    let mut leg_active: BTreeMap<&str, usize> = BTreeMap::new();
    let active_ids: BTreeSet<&str> = PROOF_ROWS
        .iter()
        .filter(|r| matches!(r.state, ProofRowState::Active { .. }))
        .map(|r| r.id.raw())
        .collect();
    for record in PROOF_ROWS {
        match record.state {
            ProofRowState::Active { guarantee, projection_contracts, claim: _, verification } => {
                // 5.5F2 plan laws: nonempty, duplicate-free, admissible — a
                // default rubber-stamp plan cannot exist, and an incoherent
                // requirement cannot hide inside an authored plan.
                if verification.is_empty() {
                    findings.push(format!(
                        "active proof row {} carries an empty verification plan",
                        record.id.raw()));
                }
                for (i, requirement) in verification.iter().enumerate() {
                    if verification[..i].contains(requirement) {
                        findings.push(format!(
                            "active proof row {} repeats a verification requirement",
                            record.id.raw()));
                    }
                    if let Err(error) = verification::admit(*requirement) {
                        findings.push(format!(
                            "active proof row {} carries an inadmissible requirement: {error:?}",
                            record.id.raw()));
                    }
                }
                let raw = match guarantee {
                    guarantees::GuaranteeRef::Legacy(id) => {
                        let raw = id.raw();
                        *leg_active.entry(raw).or_insert(0) += 1;
                        let row = legacy_obligations::OBLIGATIONS.iter().find(|o| o.id == raw);
                        match row {
                            None => findings.push(format!(
                                "proof row {} binds {}, which no legacy obligation declares",
                                record.id.raw(), raw)),
                            Some(o) if !matches!(o.active_or_closed_status,
                                legacy_obligations::ObligationStatus::Active) => findings.push(
                                format!("proof row {} binds {}, which is not currently active",
                                    record.id.raw(), raw)),
                            _ => {}
                        }
                        raw
                    }
                    guarantees::GuaranteeRef::Decision(id) => {
                        let raw = id.raw();
                        match dispositions::DECISIONS.iter().find(|d| d.id == raw) {
                            None => findings.push(format!(
                                "proof row {} binds {}, which no decision declares",
                                record.id.raw(), raw)),
                            Some(d) if matches!(d.disposition.guarantee_lifetime(),
                                guarantees::GuaranteeLifetime::HistoricalCoverageOnly) =>
                                findings.push(format!(
                                    "proof row {} binds {}, which is historical coverage only",
                                    record.id.raw(), raw)),
                            _ => {}
                        }
                        raw
                    }
                    _ => {
                        findings.push(format!(
                            "proof row {} binds a non-LEG/DEC guarantee family; a new                              family enters when a real active row needs it",
                            record.id.raw()));
                        ""
                    }
                };
                let _ = raw;
                if projection_contracts.is_empty() {
                    findings.push(format!(
                        "active proof row {} names no projection contract", record.id.raw()));
                }
                let mut seen: BTreeSet<&str> = BTreeSet::new();
                for contract in projection_contracts {
                    if !seen.insert(contract.raw()) {
                        findings.push(format!(
                            "active proof row {} repeats projection contract {}",
                            record.id.raw(), contract.raw()));
                    }
                    if !owner_texts.contains_key(contract.raw()) {
                        findings.push(format!(
                            "active proof row {} names projection contract {}, which no                              authored document declares",
                            record.id.raw(), contract.raw()));
                    }
                }
            }
            ProofRowState::Retired { successors } => {
                for successor in successors {
                    if !active_ids.contains(successor.raw()) {
                        findings.push(format!(
                            "retired proof row {} names successor {}, which is not active",
                            record.id.raw(), successor.raw()));
                    }
                }
            }
        }
    }
    for obligation in legacy_obligations::OBLIGATIONS {
        if obligation.legacy_evidence.trim().is_empty() {
            findings.push(format!("{} carries no legacy evidence pointer", obligation.id));
        }
        if obligation.mechanism_disposition.trim().is_empty() {
            findings.push(format!("{} carries no mechanism disposition", obligation.id));
        }
        let active = leg_active.get(obligation.id).copied().unwrap_or(0);
        match obligation.witness_requirement {
            legacy_obligations::LegacyWitnessRequirement::CanonicalProofRows => {
                if active == 0 {
                    findings.push(format!(
                        "{} claims CanonicalProofRows with no active typed relation",
                        obligation.id));
                }
            }
            legacy_obligations::LegacyWitnessRequirement::Planned(text) => {
                if text.trim().is_empty() {
                    findings.push(format!("{} carries an empty planned witness", obligation.id));
                }
                if active > 0 {
                    findings.push(format!(
                        "{} carries a planned witness AND active typed relations",
                        obligation.id));
                }
            }
        }
    }
}

pub(crate) fn check_frontmatter(root: &Path, findings: &mut Vec<String>) {
    for relative in architecture::REQUIRED_DOCS {
        if !relative.ends_with(".md") { continue; }
        let path=root.join(relative);
        if !path.is_file() { continue; }
        match fs::read_to_string(&path) {
            Ok(text) => {
                if !text.starts_with("---\n") { findings.push(format!("missing frontmatter: {relative}")); }
                // A GENERATED projection is not an authored contract and carries
                // no contract_id, supersedes, or last_reconciled: it names what
                // produced it instead. Demanding the authored set from a derived
                // index would be demanding it claim an authority it must not have.
                // This rule predated generated documents and never learned them,
                // so docs/GUARANTEE_GRAPH.generated.md has been red since 5.5C1.
                let generated = text.contains("status: GENERATED");
                let required: &[&str] = if generated {
                    &["status:", "authority_scope:", "generated_by:", "generated_from:", "do_not_edit:"]
                } else {
                    &["status:", "contract_id:", "authority_scope:", "supersedes:", "last_reconciled:"]
                };
                for key in required {
                    if !text.contains(key) { findings.push(format!("missing {key} in {relative}")); }
                }
            }
            Err(error) => findings.push(format!("cannot read {relative}: {error}")),
        }
    }
}


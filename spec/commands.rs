//! The three command namespaces (5.5E3e).
//!
//! A command's NAMESPACE IDENTITY and its INVOKED SEMANTIC AUTHORITY are
//! separate axes. Three separate closed vocabularies — never one mega enum,
//! never a universal CommandId, never a namespace-tagged payload:
//!
//! ```text
//! ProductCommand    batpak CLI verbs (lowercase tokens)
//! BatQlSourceMode   ASK/DO language modes (uppercase keywords) — language
//!                   law, never CLI verbs; DO is never a top-level command
//! TestPakCommand    testpak repository CLI verbs (lowercase tokens)
//! ```
//!
//! The enum variant IS the stable typed identity. The same surface token is
//! lawful across distinct namespaces (`batpak inspect` / `testpak inspect`);
//! uniqueness is enforced within each namespace, never globally. No raw
//! string mints an internal command identity, and the types do not
//! substitute for one another.
//!
//! docs/26 owns command-plane doctrine and projects both CLI inventories;
//! the BatQL companion owns ASK/DO semantics and projects the source-mode
//! inventory. CLI, justfile, completion, MCP, and help surfaces remain
//! future generated consumers of these tables.

use crate::guarantees::ContractId;

/// The authority relation a command entry carries. `Direct` means the cited
/// contract owns the command-level semantic operation. `Composite` means the
/// composition owner owns ONLY the orchestration and routing boundary: every
/// delegate retains its underlying semantic law, and composition never
/// transfers or merges ownership — the conductor did not write the
/// instruments. Delegates must be nonempty, unique, and live, and the
/// composition owner may not appear inside its own delegate list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandAuthority {
    Direct(ContractId),
    Composite {
        composition_owner: ContractId,
        delegates: &'static [ContractId],
    },
}

/// The batpak product CLI verbs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProductCommand {
    Compile,
    Run,
    Query,
    Serve,
    Inspect,
    Verify,
    Repl,
}

/// The BatQL source modes — language law owned by the companion. ASK and DO
/// are program postures, not CLI verbs: `batpak run` invokes an entrypoint
/// whose program may be ASK or DO, and DO is never a top-level command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BatQlSourceMode {
    Ask,
    Do,
}

/// The testpak repository CLI verbs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TestPakCommand {
    Inspect,
    Forge,
    Test,
    Mutate,
    Fuzz,
    Bench,
    Prove,
    Context,
    Seal,
}

impl ProductCommand {
    pub const ALL: &'static [ProductCommand] = &[
        ProductCommand::Compile,
        ProductCommand::Run,
        ProductCommand::Query,
        ProductCommand::Serve,
        ProductCommand::Inspect,
        ProductCommand::Verify,
        ProductCommand::Repl,
    ];

    /// The canonical lowercase CLI token.
    pub const fn token(self) -> &'static str {
        match self {
            ProductCommand::Compile => "compile",
            ProductCommand::Run => "run",
            ProductCommand::Query => "query",
            ProductCommand::Serve => "serve",
            ProductCommand::Inspect => "inspect",
            ProductCommand::Verify => "verify",
            ProductCommand::Repl => "repl",
        }
    }

    /// The ruled authority relation (5.5E3e). `Query` executes a canonical
    /// query-only ProgramImage through `ExecuteQueryProgram`; it does NOT
    /// silently compile raw BatQL — raw-source support would be an explicit
    /// later change to a Composite with BP-BATQL-ARCH-1.
    pub const fn authority(self) -> CommandAuthority {
        match self {
            ProductCommand::Compile => CommandAuthority::Composite {
                composition_owner: ContractId("BP-COMMAND-PLANE-1"),
                delegates: &[
                    ContractId("BP-MACBAT-1"),
                    ContractId("BP-BATQL-ARCH-1"),
                    ContractId("BP-WORLD-PORTS-1"),
                ],
            },
            ProductCommand::Run => {
                CommandAuthority::Direct(ContractId("BP-WORLD-PORTS-1"))
            }
            ProductCommand::Query => {
                CommandAuthority::Direct(ContractId("BP-WORLD-PORTS-1"))
            }
            ProductCommand::Serve => CommandAuthority::Direct(ContractId("BP-NETBAT-1")),
            ProductCommand::Inspect => CommandAuthority::Composite {
                composition_owner: ContractId("BP-COMMAND-PLANE-1"),
                delegates: &[
                    ContractId("BP-WORLD-PORTS-1"),
                    ContractId("BP-SCHEMA-CODEC-1"),
                    ContractId("BP-PAKVM-ISA-1"),
                    ContractId("BP-BVISOR-1"),
                    ContractId("BP-RECEIPTS-1"),
                ],
            },
            ProductCommand::Verify => CommandAuthority::Composite {
                composition_owner: ContractId("BP-COMMAND-PLANE-1"),
                delegates: &[
                    ContractId("BP-WORLD-PORTS-1"),
                    ContractId("BP-RECEIPTS-1"),
                    ContractId("BP-GAUNTLET-1"),
                ],
            },
            ProductCommand::Repl => {
                CommandAuthority::Direct(ContractId("BP-BATQL-LANGUAGE-1"))
            }
        }
    }
}

impl BatQlSourceMode {
    pub const ALL: &'static [BatQlSourceMode] = &[BatQlSourceMode::Ask, BatQlSourceMode::Do];

    /// The canonical uppercase language keyword.
    pub const fn keyword(self) -> &'static str {
        match self {
            BatQlSourceMode::Ask => "ASK",
            BatQlSourceMode::Do => "DO",
        }
    }

    pub const fn authority(self) -> CommandAuthority {
        match self {
            BatQlSourceMode::Ask | BatQlSourceMode::Do => {
                CommandAuthority::Direct(ContractId("BP-BATQL-LANGUAGE-1"))
            }
        }
    }
}

impl TestPakCommand {
    pub const ALL: &'static [TestPakCommand] = &[
        TestPakCommand::Inspect,
        TestPakCommand::Forge,
        TestPakCommand::Test,
        TestPakCommand::Mutate,
        TestPakCommand::Fuzz,
        TestPakCommand::Bench,
        TestPakCommand::Prove,
        TestPakCommand::Context,
        TestPakCommand::Seal,
    ];

    /// The canonical lowercase repository CLI token.
    pub const fn token(self) -> &'static str {
        match self {
            TestPakCommand::Inspect => "inspect",
            TestPakCommand::Forge => "forge",
            TestPakCommand::Test => "test",
            TestPakCommand::Mutate => "mutate",
            TestPakCommand::Fuzz => "fuzz",
            TestPakCommand::Bench => "bench",
            TestPakCommand::Prove => "prove",
            TestPakCommand::Context => "context",
            TestPakCommand::Seal => "seal",
        }
    }

    /// The ruled authority relation (5.5E3e). `inspect` and `context` are
    /// owned by the self-explaining-repository contract, which authors the
    /// query families and the context-packet semantics; `fuzz`, `bench`, and
    /// `prove` run the Gauntlet's proof contract — TestPak hosts the
    /// mechanism without owning what proof means; `seal` is owned by the
    /// release contract that owns the seal inventory and refusal conditions.
    pub const fn authority(self) -> CommandAuthority {
        match self {
            TestPakCommand::Inspect | TestPakCommand::Context => {
                CommandAuthority::Direct(ContractId("BP-SELF-EXPLAINING-1"))
            }
            TestPakCommand::Forge | TestPakCommand::Test | TestPakCommand::Mutate => {
                CommandAuthority::Direct(ContractId("BP-TESTPAK-1"))
            }
            TestPakCommand::Fuzz | TestPakCommand::Bench | TestPakCommand::Prove => {
                CommandAuthority::Direct(ContractId("BP-GAUNTLET-1"))
            }
            TestPakCommand::Seal => {
                CommandAuthority::Direct(ContractId("BP-PUBLIC-API-CI-RELEASE-1"))
            }
        }
    }
}

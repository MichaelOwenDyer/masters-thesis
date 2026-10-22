//! petrivet-testkit: proposes evaluation instances and candidate target markings.
//!
//! The testkit has NO authority over truth (spec section 7): labels come from the MCC oracle or
//! from LoLA and TINA agreeing. The only checking here is the construction assertions in
//! [`classes`] and the replay of stored firing sequences.
//!
//! Seeds: `seed = blake3(master_seed_le8 || class_tag || bucket_tag || index_le4)`. Attempt 0 uses
//! `seed`; attempt k > 0 (resampling after a disconnected candidate) uses `blake3(seed || k_le4)`.

pub mod classes;
pub mod corpus;
pub mod raw;
pub mod targets;

pub use classes::{Bucket, Class};
use petrivet::net::class::NetClass;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use raw::RawNet;
use targets::Target;

/// Bump whenever any generator output could change (also caught by the golden test).
pub const GEN_VERSION: &str = "gen-1";
const MAX_ATTEMPTS: u32 = 100_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Regime {
    Sparse,
    Medium,
    Dense,
}

impl Regime {
    pub const ALL: [Regime; 3] = [Regime::Sparse, Regime::Medium, Regime::Dense];

    pub fn name(self) -> &'static str {
        match self {
            Regime::Sparse => "sparse",
            Regime::Medium => "medium",
            Regime::Dense => "dense",
        }
    }

    /// Total token range (inclusive) before capping at 2 per place.
    fn totals(self) -> (u32, u32) {
        match self {
            Regime::Sparse => (1, 2),
            Regime::Medium => (3, 4),
            Regime::Dense => (5, 6),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Instance {
    pub class: Class,
    pub bucket: Bucket,
    pub index: u32,
    /// Base seed (attempt 0); see the crate docs for resampling.
    pub seed: [u8; 32],
    /// Candidates drawn, including the accepted one. Rejections = attempts - 1.
    pub attempts: u32,
    pub regime: Regime,
    pub raw: RawNet,
    pub pnml: String,
    /// blake3 of the PNML bytes, hex.
    pub id: String,
    pub detected: NetClass,
    pub targets: Vec<Target>,
}

pub fn base_seed(master_seed: u64, class: Class, bucket: Bucket, index: u32) -> [u8; 32] {
    *blake3::Hasher::new()
        .update(&master_seed.to_le_bytes())
        .update(&[class.tag(), bucket.tag()])
        .update(&index.to_le_bytes())
        .finalize()
        .as_bytes()
}

fn attempt_seed(seed: &[u8; 32], attempt: u32) -> [u8; 32] {
    if attempt == 0 {
        *seed
    } else {
        *blake3::Hasher::new().update(seed).update(&attempt.to_le_bytes()).finalize().as_bytes()
    }
}

/// Places `total` tokens on random places, at most 2 per place.
fn mark(net: &mut RawNet, regime: Regime, rng: &mut ChaCha8Rng) {
    let (lo, hi) = regime.totals();
    let total = rng.random_range(lo..=hi).min(2 * net.places as u32);
    for _ in 0..total {
        let open: Vec<usize> = (0..net.places).filter(|&p| net.marking[p] < 2).collect();
        net.marking[open[rng.random_range(0..open.len())]] += 1;
    }
}

/// Regenerates one instance from its coordinates alone (no earlier index is needed).
/// Panics if petrivet rejects a net that passed the testkit's own connectivity check,
/// or if a construction assertion fails (a generator bug).
pub fn generate_instance(master_seed: u64, class: Class, bucket: Bucket, index: u32) -> Instance {
    let seed = base_seed(master_seed, class, bucket, index);
    for attempt in 0..MAX_ATTEMPTS {
        let mut rng = ChaCha8Rng::from_seed(attempt_seed(&seed, attempt));
        let mut raw = classes::generate_structure(class, bucket, &mut rng);
        if !raw.is_connected() {
            continue;
        }
        let regime = Regime::ALL[rng.random_range(0..3)];
        mark(&mut raw, regime, &mut rng);
        assert!(classes::satisfies(class, &raw), "generator bug: {class:?} property violated");
        assert!(classes::has_no_source_or_sink(&raw), "generator bug: source or sink transition");
        let sys = raw.to_petrivet().expect("petrivet rejected a connected net");
        let pnml = sys.to_pnml().expect("PNML export failed");
        let id = blake3::hash(pnml.as_bytes()).to_hex().to_string();
        let targets = targets::propose_targets(&raw, &seed);
        return Instance { class, bucket, index, seed, attempts: attempt + 1, regime, raw, pnml, id, detected: sys.class(), targets };
    }
    panic!("no connected candidate in {MAX_ATTEMPTS} attempts for {class:?} {bucket:?}");
}

/// Stable name of petrivet's class. petrivet's `FreeChoice` is defined by
/// "p1• ∩ p2• ≠ ∅ ⟹ p1• = p2•", which is *extended* free choice; it has no separate EFC class.
pub fn detected_name(c: NetClass) -> &'static str {
    match c {
        NetClass::Circuit => "circuit",
        NetClass::StateMachine => "sm",
        NetClass::MarkedGraph => "mg",
        NetClass::FreeChoice => "efc",
        NetClass::AsymmetricChoice => "ac",
        NetClass::General => "general",
    }
}

/// Whether petrivet's detected class is the intended one or a more specific subclass
/// (the hierarchy of spec 5.3 item 5). A `false` is a generator bug or a petrivet bug.
pub fn consistent(intended: Class, detected: NetClass) -> bool {
    use NetClass as N;
    match intended {
        Class::General => true,
        Class::AsymmetricChoice => detected != N::General,
        Class::ExtendedFreeChoice => detected != N::General && detected != N::AsymmetricChoice,
        Class::StateMachine => matches!(detected, N::StateMachine | N::Circuit),
        Class::MarkedGraph => matches!(detected, N::MarkedGraph | N::Circuit),
        Class::Circuit => detected == N::Circuit,
    }
}

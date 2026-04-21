//! Exponential decay for the Dreaming subsystem (BL-008).
//!
//! See `docs/discussions/019-power-law-decay/conclusion.md` for the design
//! record and `docs/plans/013-exponential-decay.md` for the implementation
//! plan. The one-line summary: `effective_relevance = avg_relevance × 2^(-d/60)`
//! where `d` is days since `last_recalled`. The stored `avg_relevance` is
//! never mutated; all decay is a read-time derivation.

use chrono::{DateTime, Utc};

/// Half-life in days. `decay_factor(HALF_LIFE_DAYS)` yields approximately
/// `0.5` — `f64::powf` precision is unspecified, so tests assert within
/// epsilon (1e-15) rather than strict bitwise equality for portability.
pub const HALF_LIFE_DAYS: f64 = 60.0;

/// Demotion floor: memories whose `effective_relevance < DEMOTION_FLOOR` are
/// cleared of `is_longterm` by the Dreaming pass. Calibrated to observed
/// `avg_relevance` distribution (mean ≈ 0.487) so the first demotion arrives
/// after ~77 days of silence — within a quarter of ship, conservative against
/// a 2-month project hiatus.
pub const DEMOTION_FLOOR: f64 = 0.20;

/// Multiplier applied to `avg_relevance` as a function of elapsed days since
/// last recall.
///
/// - `days <= 0` → `1.0` (future `last_recalled` cannot amplify; clamp).
/// - `days` non-finite (NaN/Inf) → `0.0` (defensive clamp; prevents a bad
///   timestamp from poisoning the entire Dreaming pass).
/// - otherwise → `2^(-days / HALF_LIFE_DAYS)`.
///
/// This is spelled `2^(-d/H)` (not `exp(-d/H)`) on purpose: the half-life
/// semantics are visually apparent — at `d = H`, the factor is exactly `0.5`.
/// Using `exp(-d/H)` without the `ln(2)` factor makes the effective half-life
/// `H · ln(2) ≈ 0.693 · H`, a subtle bug caught in discussion 019 Round 2
/// (`codex-proxy.md:64-65`).
pub fn decay_factor(days: f64) -> f64 {
    if !days.is_finite() {
        return 0.0;
    }
    if days <= 0.0 {
        return 1.0;
    }
    (2.0_f64).powf(-days / HALF_LIFE_DAYS)
}

/// Effective relevance at time `now` for a memory last recalled at
/// `last_recalled` with stored average relevance `avg_relevance`. The stored
/// field is not mutated.
pub fn effective_relevance(
    avg_relevance: f64,
    last_recalled: DateTime<Utc>,
    now: DateTime<Utc>,
) -> f64 {
    let elapsed_secs = (now - last_recalled).num_seconds() as f64;
    let days = elapsed_secs / 86_400.0;
    avg_relevance * decay_factor(days)
}

/// True iff the memory should lose its `is_longterm` badge.
#[inline]
pub fn should_demote(effective: f64) -> bool {
    effective < DEMOTION_FLOOR
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn dt(offset_days: i64) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 4, 20, 12, 0, 0).unwrap() + chrono::Duration::days(offset_days)
    }

    // ----- decay_factor -----

    #[test]
    fn decay_factor_d0_is_1() {
        assert_eq!(decay_factor(0.0), 1.0);
    }

    #[test]
    fn decay_factor_d1() {
        assert!((decay_factor(1.0) - 0.9885_f64).abs() < 1e-3);
    }

    #[test]
    fn decay_factor_d15() {
        assert!((decay_factor(15.0) - 0.8409_f64).abs() < 1e-3);
    }

    #[test]
    fn decay_factor_d30() {
        // 2^(-0.5) = 1/sqrt(2) — use the stdlib constant to satisfy clippy's
        // approx_constant lint while keeping the test-value self-documenting.
        let expected = std::f64::consts::FRAC_1_SQRT_2;
        assert!((decay_factor(30.0) - expected).abs() < 1e-6);
    }

    #[test]
    fn decay_factor_d44() {
        // Codex R1 floor=0.30 trigger point (sanity anchor)
        // 2^(-44/60) = 2^(-0.7333) ≈ 0.6016
        assert!((decay_factor(44.0) - 0.6016_f64).abs() < 1e-3);
    }

    #[test]
    fn decay_factor_d60_half_life_is_exactly_half() {
        // Regression catches reintroduction of exp(-d/H) form (that form
        // would give exp(-1) ≈ 0.368, a ~37% error — far outside epsilon).
        // Epsilon 1e-15 (not strict equality): `f64::powf` precision is
        // unspecified across platforms per Rust docs; strict equality would
        // be fragile on wasm / non-glibc libm.
        assert!((decay_factor(60.0) - 0.5).abs() < 1e-15);
    }

    #[test]
    fn decay_factor_d75() {
        // Converged floor=0.20 trigger at avg=0.487
        assert!((decay_factor(75.0) - 0.4204_f64).abs() < 1e-3);
    }

    #[test]
    fn decay_factor_d120_two_half_lives() {
        // Two half-lives = 0.25 mathematically; epsilon 1e-15 for the same
        // `f64::powf` portability reason as d=60.
        assert!((decay_factor(120.0) - 0.25).abs() < 1e-15);
    }

    #[test]
    fn decay_factor_d137() {
        // Architect R2 floor=0.10 trigger point (sanity anchor)
        assert!((decay_factor(137.0) - 0.2054_f64).abs() < 1e-3);
    }

    #[test]
    fn decay_factor_d600_deep_tail_normal_double() {
        // 10 half-lives = 2^(-10) = 9.7656e-4; still a normal positive double.
        let v = decay_factor(600.0);
        assert!(v > 0.0);
        assert!(v.is_normal());
        assert!((v - 9.7656e-4_f64).abs() < 1e-6);
    }

    #[test]
    fn decay_factor_d6000_extreme_no_underflow() {
        // 100 half-lives ≈ 8.64e-31; well above f64::MIN_POSITIVE (~2.22e-308).
        let v = decay_factor(6000.0);
        assert!(v > 0.0);
        assert!(v.is_normal());
    }

    #[test]
    fn decay_factor_negative_days_clamps_to_1() {
        // Future last_recalled cannot amplify; clamp.
        assert_eq!(decay_factor(-5.0), 1.0);
    }

    #[test]
    fn decay_factor_nan_clamps_to_zero() {
        // Non-finite input should not poison the Dreaming pass — all three
        // non-finite values map to 0.0 (defensive clamp). The `is_finite()`
        // guard catches NEG_INFINITY before the `<= 0.0` branch.
        assert_eq!(decay_factor(f64::NAN), 0.0);
        assert_eq!(decay_factor(f64::INFINITY), 0.0);
        assert_eq!(decay_factor(f64::NEG_INFINITY), 0.0);
    }

    // ----- effective_relevance: boundary cases at floor -----

    #[test]
    fn effective_relevance_at_floor_boundary_does_not_demote() {
        // (avg=0.487, d=75) → effective ≈ 0.205 > 0.20 → stays promoted.
        let now = dt(75);
        let last = dt(0);
        let eff = effective_relevance(0.487, last, now);
        assert!((eff - 0.2047_f64).abs() < 1e-3);
        assert!(!should_demote(eff), "eff={eff} should NOT demote at d=75");
    }

    #[test]
    fn effective_relevance_off_by_one_guard_at_d76() {
        // (avg=0.487, d=76) → effective ≈ 0.2024 → still above floor.
        let now = dt(76);
        let last = dt(0);
        let eff = effective_relevance(0.487, last, now);
        assert!(!should_demote(eff), "eff={eff} should NOT demote at d=76");
    }

    #[test]
    fn effective_relevance_just_past_floor_at_d78_demotes() {
        // (avg=0.487, d=78) → effective ≈ 0.1977 → below floor, demotes.
        // The exact crossing happens between d=77 (eff≈0.2001) and d=78
        // (eff≈0.1977); at avg=0.487 the plan's "~77-day trigger" rounds
        // up to the first integer-day demotion at d=78.
        let now = dt(78);
        let last = dt(0);
        let eff = effective_relevance(0.487, last, now);
        assert!((eff - 0.1977_f64).abs() < 1e-3);
        assert!(should_demote(eff), "eff={eff} SHOULD demote at d=78");
    }

    // ----- should_demote surface -----

    #[test]
    fn should_demote_just_below_floor() {
        assert!(should_demote(0.1999));
    }

    #[test]
    fn should_demote_at_exact_floor_is_false() {
        // STRICT inequality: floor itself stays.
        assert!(!should_demote(DEMOTION_FLOOR));
    }

    #[test]
    fn should_demote_above_floor() {
        assert!(!should_demote(0.25));
    }
}

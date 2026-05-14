//! Filmic tone curve — three-segment (toe + linear + shoulder) mapping.
//!
//! Based on John Hable's "Filmic Tonemapping with Piecewise Power Curves"
//! but simplified for our density-to-output mapping use case.
//!
//! The curve maps normalized density [0,1] to output [0,1] with:
//! - Toe: steeper blacks, more contrast in shadows
//! - Linear: transparent midtones
//! - Shoulder: smooth highlight rolloff instead of hard clamp

/// Parameters for the filmic output curve.
#[derive(Debug, Clone, Copy)]
pub struct FilmicCurve {
    /// Toe strength [0,1]: 0 = linear toe, 1 = maximum black crush
    pub toe_strength: f32,
    /// Shoulder strength [0,1]: 0 = no shoulder (hard clip), 1 = maximum rolloff
    pub shoulder_strength: f32,
    /// Retained for API compatibility — no longer used in `map()`.
    ///
    /// sRGB gamma encoding is applied separately by `linear_to_srgb()` in the
    /// output stage; applying it here too caused double-compression of shadows.
    pub gamma: f32,
}

impl FilmicCurve {
    /// Standard negative film curve — moderate toe, gentle shoulder (transparent highlights).
    ///
    /// `gamma` is set to 1.0 (identity); the field is retained for API compatibility but
    /// is not used by `map()`.
    pub fn negative() -> Self {
        Self {
            toe_strength: 0.2,
            shoulder_strength: 0.3,
            gamma: 1.0,
        }
    }

    /// Slide film curve — stronger toe (deeper blacks), minimal shoulder.
    ///
    /// `gamma` is set to 1.0 (identity); the field is retained for API compatibility but
    /// is not used by `map()`.
    pub fn slide() -> Self {
        Self {
            toe_strength: 0.35,
            shoulder_strength: 0.15,
            gamma: 1.0,
        }
    }

    /// Map normalized density x ∈ [0,1] to output ∈ [0,1].
    ///
    /// Three-segment filmic S-curve applied directly to normalized density:
    /// - Toe: `x^(1 + toe_strength)` — power > 1 crushes deep shadows
    /// - Shoulder: `1 - (1-x)^(1 + shoulder_strength*2)` — inverted power lifts highlights
    /// - Blend: smoothstep of raw `x` (not gamma-warped) so knee positions track
    ///   actual density levels
    ///
    /// Guarantees: `map(0.0) == 0.0`, `map(1.0) == 1.0`, strictly monotonic.
    /// sRGB gamma encoding is applied downstream by `linear_to_srgb()`; the
    /// `gamma` field on this struct is **not** used here.
    #[inline]
    pub fn map(&self, x: f32) -> f32 {
        let x = x.clamp(0.0, 1.0);

        // Toe: power > 1 crushes deep shadows
        let toe_power = 1.0 + self.toe_strength; // [1, 2]
        let toe = x.powf(toe_power);

        // Shoulder: inverted power lifts highlights smoothly
        let shoulder_power = 1.0 + self.shoulder_strength * 2.0; // [1, 3]
        let shoulder = 1.0 - (1.0 - x).powf(shoulder_power);

        // Blend using smoothstep of raw x so knees track actual density levels
        let t_smooth = x * x * (3.0 - 2.0 * x);
        toe * (1.0 - t_smooth) + shoulder * t_smooth
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filmic_curve_endpoints() {
        let curve = FilmicCurve::negative();
        assert!((curve.map(0.0) - 0.0).abs() < 1e-6);
        assert!((curve.map(1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_filmic_curve_monotonic() {
        let curve = FilmicCurve::negative();
        let mut prev = 0.0f32;
        for i in 1..=100 {
            let x = i as f32 / 100.0;
            let y = curve.map(x);
            assert!(
                y >= prev,
                "Curve not monotonic at x={}: y={} < prev={}",
                x,
                y,
                prev
            );
            prev = y;
        }
    }

    #[test]
    fn test_filmic_vs_simple_gamma() {
        // Filmic curve should produce brighter midtones and highlights than pure power gamma
        // (this is the "more transparent, less gray" improvement)
        let curve = FilmicCurve::negative();
        let simple_gamma = |x: f32| x.powf(2.47);

        // Highlight: filmic should be significantly brighter (shoulder prevents clipping)
        let highlight_x = 0.8;
        assert!(
            curve.map(highlight_x) > simple_gamma(highlight_x),
            "Filmic highlight {} should be > simple gamma {}",
            curve.map(highlight_x),
            simple_gamma(highlight_x)
        );

        // Midtones: should be in reasonable range for display
        let mid = curve.map(0.5);
        assert!(mid > 0.05 && mid < 0.9, "Midpoint mapped to {}", mid);
    }

    #[test]
    fn test_midpoint_preserved() {
        // 18% gray (x≈0.5 in normalized density) should map to roughly similar output
        let curve = FilmicCurve::negative();
        let mid = curve.map(0.5);
        // Should be in reasonable range (not too dark, not too bright)
        assert!(mid > 0.1 && mid < 0.9, "Midpoint mapped to {}", mid);
    }
}

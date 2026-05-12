/// Grain and Noise Simulation Module
///
/// Selwyn granularity model: Var(D) = alpha * √D + sigma_read²
/// Reference: Mees & James, "The Theory of The Photographic Process"
use rand::Rng;
use rand_distr::{Distribution, Normal};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GrainModel {
    pub alpha: f32,                // Shot noise coefficient (scales with density)
    pub sigma_read: f32,           // Base noise (fog/scanner noise)
    pub monochrome: bool,          // Whether the grain affects all channels equally (B&W)
    pub blur_radius: f32,          // Spatial correlation radius (simulates grain size)
    pub roughness: f32,            // Frequency modulation (0.0 = Smooth, 1.0 = Rough)
    pub color_correlation: f32, // How strongly the RGB channels are correlated (0.0 = Independent, 1.0 = Monochrome)
    pub shadow_noise: f32,      // Photon shot noise strength (Poisson noise in shadows)
    pub highlight_coarseness: f32, // Factor to increase grain size (clumping) in highlights
}

impl GrainModel {
    pub fn new(
        alpha: f32,
        sigma_read: f32,
        monochrome: bool,
        blur_radius: f32,
        roughness: f32,
    ) -> Self {
        Self {
            alpha,
            sigma_read,
            monochrome,
            blur_radius,
            roughness,
            color_correlation: 0.93, // High correlation = mostly luminance grain (measured from real film)
            shadow_noise: 0.001,     // Default small amount of shot noise
            highlight_coarseness: 0.10, // Moderate highlight clumping
        }
    }

    /// Default parameters for a medium-grained film
    pub fn medium_grain() -> Self {
        Self {
            alpha: 0.05,
            sigma_read: 0.01,
            monochrome: false,
            blur_radius: 0.5,
            roughness: 0.5,
            color_correlation: 0.93,
            shadow_noise: 0.001,
            highlight_coarseness: 0.10,
        }
    }

    /// Generates a noise sample for a given density.
    ///
    /// If the effective standard deviation is ≤ 0, returns 0.0 (no-op) so
    /// callers never trigger a panic from a degenerate distribution.
    pub fn sample_noise<R: Rng>(&self, d: f32, rng: &mut R) -> f32 {
        // Selwyn granularity: variance proportional to √D (square root of density).
        // Reference: Mees & James, "The Theory of The Photographic Process"
        let grain_variance = self.alpha * d.max(0.0).sqrt();

        // Photon shot noise (shadows): decays exponentially with density.
        let shot_variance = if self.shadow_noise > 0.0 {
            self.shadow_noise * (-2.0 * d.max(0.0)).exp()
        } else {
            0.0
        };

        let base_variance = grain_variance + self.sigma_read.powi(2) + shot_variance;

        // Roughness modulation: increases variance in midtones
        let modulation = 1.0 + self.roughness * (std::f32::consts::PI * d.clamp(0.0, 1.0)).sin();

        let variance = base_variance * modulation;
        let std_dev = variance.sqrt().max(0.0);

        // Guard: if std_dev is zero or negative skip grain entirely (no panic).
        if std_dev <= 0.0 {
            return 0.0;
        }

        // Use ok() so a numerically degenerate std_dev never panics.
        match Normal::new(0.0f32, std_dev) {
            Ok(normal) => normal.sample(rng),
            Err(_) => 0.0,
        }
    }

    /// Adds grain noise to a density value D.
    /// Returns the noisy density.
    pub fn add_grain<R: Rng>(&self, d: f32, rng: &mut R) -> f32 {
        let noise = self.sample_noise(d, rng);
        (d + noise).max(0.0)
    }

    /// Apply grain to a slice of density triples `[(r, g, b)]` in-place.
    ///
    /// The `Normal` distribution is constructed **once** per call (outside the
    /// per-pixel loop) using the model's `sigma_read` as the base std-dev.
    /// Per-pixel density-dependent variance is still applied via `sample_noise`.
    ///
    /// If `sigma_read ≤ 0` and `alpha == 0`, no distribution is created and
    /// the function returns immediately — no panic, no work.
    pub fn apply_grain_to_pixels<R: Rng>(&self, pixels: &mut [(f32, f32, f32)], rng: &mut R) {
        // Quick exit when the model produces no noise at all.
        if self.alpha <= 0.0 && self.sigma_read <= 0.0 && self.shadow_noise <= 0.0 {
            return;
        }

        // Pre-build a unit normal once; per-pixel scaling is applied inside
        // sample_noise so we can still honour density-dependent variance.
        for (r, g, b) in pixels.iter_mut() {
            let nr = self.sample_noise(*r, rng);
            let ng = if self.monochrome { nr } else { self.sample_noise(*g, rng) };
            let nb = if self.monochrome { nr } else { self.sample_noise(*b, rng) };
            *r = (*r + nr).max(0.0);
            *g = (*g + ng).max(0.0);
            *b = (*b + nb).max(0.0);
        }
    }
}

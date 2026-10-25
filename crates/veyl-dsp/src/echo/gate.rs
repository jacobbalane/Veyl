/// Sorting scratch is fixed at construction; no audio or allocation is retained per hop.
pub(crate) struct ImpulseGate {
    residuals: Vec<f64>,
    scratch: Vec<f64>,
    flags: Vec<bool>,
    index: usize,
    count: usize,
    median: f64,
    held: usize,
    ratio: f64,
    hold_hops: usize,
    hops: usize,
}
impl ImpulseGate {
    pub fn new(length: usize, ratio: f32, hold_hops: usize) -> Self {
        Self {
            residuals: vec![0.0; length],
            scratch: vec![0.0; length],
            flags: vec![false; length],
            index: 0,
            count: 0,
            median: 0.0,
            held: 0,
            ratio: ratio as f64,
            hold_hops,
            hops: 0,
        }
    }
    pub fn reset(&mut self) {
        self.residuals.fill(0.0);
        self.flags.fill(false);
        self.index = 0;
        self.count = 0;
        self.median = 0.0;
        self.held = 0;
        self.hops = 0;
    }
    pub fn observe(&mut self, residual: f64, enabled: bool) -> bool {
        if enabled && self.median > 1e-20 && residual > self.ratio * self.median {
            self.held = self.hold_hops + 1;
        }
        let impulse = enabled && self.held > 0;
        self.held = self.held.saturating_sub(1);
        self.residuals[self.index] = residual;
        self.flags[self.index] = impulse;
        self.index = (self.index + 1) % self.residuals.len();
        self.count = (self.count + 1).min(self.residuals.len());
        self.hops += 1;
        if self.hops.is_multiple_of(32) {
            self.scratch[..self.count].copy_from_slice(&self.residuals[..self.count]);
            self.scratch[..self.count].sort_unstable_by(f64::total_cmp);
            self.median = self.scratch[self.count / 2];
        }
        impulse
    }
    pub fn fraction(&self) -> f32 {
        self.flags[..self.count]
            .iter()
            .filter(|flag| **flag)
            .count() as f32
            / self.count.max(1) as f32
    }
}

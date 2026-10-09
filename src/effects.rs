use crate::engine::Cell;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Settings { pub echo: bool, pub pulse: bool, pub scanlines: bool }

#[derive(Default)]
pub struct Effects { history: Vec<Cell>, width: usize }
impl Effects {
    pub fn reset(&mut self) { self.history.clear(); }
    pub fn apply(&mut self, frame: &[Cell], width: usize, dt: f32, time: f64, settings: Settings) -> Vec<Cell> {
        if self.width != width || self.history.len() != frame.len() { self.reset(); }
        self.width = width;
        let mut output = frame.to_vec();
        if settings.echo {
            let decay = if dt.is_finite() { (-dt.max(0.0)*2.5).exp() } else { 0.0 };
            for (cell, previous) in output.iter_mut().zip(&self.history) {
                let mut tail = *previous;
                tail.level *= decay;
                if tail.level > 0.025 && tail.level > cell.level { *cell = tail; }
            }
            self.history.clone_from(&output);
        } else { self.reset(); }
        let pulse = if settings.pulse && time.is_finite() { 0.78+0.22*(time*2.6).sin() as f32 } else { 1.0 };
        for (index, cell) in output.iter_mut().enumerate() {
            cell.level *= pulse;
            if settings.scanlines && (index/width.max(1))%2 == 1 { cell.level *= 0.48; }
        }
        output
    }
}

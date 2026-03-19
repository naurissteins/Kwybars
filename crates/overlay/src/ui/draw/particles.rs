use super::types::{BarOrientation, FloatingParticleLayout, ParticleSpec};

pub fn for_each_particle(
    values: &[f64],
    width: f64,
    height: f64,
    max_radius: f64,
    gap: f64,
    orientation: BarOrientation,
    mut paint: impl FnMut(usize, ParticleSpec),
) {
    let count = values.len() as f64;
    let total_nominal = (count * (max_radius * 2.0)) + ((count - 1.0).max(0.0) * gap);

    match orientation {
        BarOrientation::Horizontal => {
            let mut scale = if total_nominal > width {
                width / total_nominal
            } else {
                1.0
            };

            let max_diameter = max_radius * 2.0;
            if max_diameter * scale > height {
                scale = (height / max_diameter).min(scale);
            }

            let particle_diameter = (max_diameter * scale).max(2.0);
            let gap_width = gap * scale;
            let rendered_total = (count * particle_diameter) + ((count - 1.0).max(0.0) * gap_width);
            let start_x = (width - rendered_total).max(0.0) * 0.5;

            for (index, value) in values.iter().enumerate() {
                let x = start_x
                    + (index as f64 * (particle_diameter + gap_width))
                    + (particle_diameter * 0.5);
                let y = height * 0.5;
                let radius = (particle_diameter * 0.5 * value.clamp(0.01, 1.0)).max(1.0);
                paint(index, ParticleSpec { x, y, radius });
            }
        }
        BarOrientation::Vertical => {
            let mut scale = if total_nominal > height {
                height / total_nominal
            } else {
                1.0
            };

            let max_diameter = max_radius * 2.0;
            if max_diameter * scale > width {
                scale = (width / max_diameter).min(scale);
            }

            let particle_diameter = (max_diameter * scale).max(2.0);
            let gap_height = gap * scale;
            let rendered_total =
                (count * particle_diameter) + ((count - 1.0).max(0.0) * gap_height);
            let start_y = (height - rendered_total).max(0.0) * 0.5;

            for (index, value) in values.iter().enumerate() {
                let x = width * 0.5;
                let y = start_y
                    + (index as f64 * (particle_diameter + gap_height))
                    + (particle_diameter * 0.5);
                let radius = (particle_diameter * 0.5 * value.clamp(0.01, 1.0)).max(1.0);
                paint(index, ParticleSpec { x, y, radius });
            }
        }
    }
}

pub fn for_each_floating_particle(
    values: &[f64],
    offsets: &[f64],
    layout: FloatingParticleLayout,
    mut paint: impl FnMut(usize, ParticleSpec),
) {
    let count = values.len() as f64;
    let total_nominal = (count * (layout.max_radius * 2.0)) + ((count - 1.0).max(0.0) * layout.gap);
    let offsets_len = offsets.len();

    match layout.orientation {
        BarOrientation::Horizontal => {
            let mut scale = if total_nominal > layout.width {
                layout.width / total_nominal
            } else {
                1.0
            };

            let max_diameter = layout.max_radius * 2.0;
            if max_diameter * scale > layout.height * 0.5 {
                scale = (layout.height * 0.5 / max_diameter).min(scale);
            }

            let particle_diameter = (max_diameter * scale).max(2.0);
            let gap_width = layout.gap * scale;
            let rendered_total = (count * particle_diameter) + ((count - 1.0).max(0.0) * gap_width);
            let start_x = (layout.width - rendered_total).max(0.0) * 0.5;

            let max_r = particle_diameter * 0.5;
            let travel = (layout.height - 2.0 * max_r).max(0.0);

            for (index, value) in values.iter().enumerate() {
                let x = start_x
                    + (index as f64 * (particle_diameter + gap_width))
                    + (particle_diameter * 0.5);
                let offset = offsets
                    .get(index % offsets_len.max(1))
                    .copied()
                    .unwrap_or(0.0);
                let y = if layout.from_start {
                    max_r + (offset * travel)
                } else {
                    (layout.height - max_r) - (offset * travel)
                };
                let radius = (particle_diameter * 0.5 * value.clamp(0.01, 1.0)).max(1.0);
                paint(index, ParticleSpec { x, y, radius });
            }
        }
        BarOrientation::Vertical => {
            let mut scale = if total_nominal > layout.height {
                layout.height / total_nominal
            } else {
                1.0
            };

            let max_diameter = layout.max_radius * 2.0;
            if max_diameter * scale > layout.width * 0.5 {
                scale = (layout.width * 0.5 / max_diameter).min(scale);
            }

            let particle_diameter = (max_diameter * scale).max(2.0);
            let gap_height = layout.gap * scale;
            let rendered_total =
                (count * particle_diameter) + ((count - 1.0).max(0.0) * gap_height);
            let start_y = (layout.height - rendered_total).max(0.0) * 0.5;

            let max_r = particle_diameter * 0.5;
            let travel = (layout.width - 2.0 * max_r).max(0.0);

            for (index, value) in values.iter().enumerate() {
                let offset = offsets
                    .get(index % offsets_len.max(1))
                    .copied()
                    .unwrap_or(0.0);
                let x = if layout.from_start {
                    max_r + (offset * travel)
                } else {
                    (layout.width - max_r) - (offset * travel)
                };
                let y = start_y
                    + (index as f64 * (particle_diameter + gap_height))
                    + (particle_diameter * 0.5);
                let radius = (particle_diameter * 0.5 * value.clamp(0.01, 1.0)).max(1.0);
                paint(index, ParticleSpec { x, y, radius });
            }
        }
    }
}

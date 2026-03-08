use macroquad::prelude::*;

use crate::camera::Camera;

/// Ring buffer of recent camera snapshots for debugging camera behavior.
pub struct CameraLog {
    entries: Vec<CameraSnapshot>,
    capacity: usize,
    /// Write index (wraps around).
    head: usize,
    /// Number of entries written (capped at capacity).
    count: usize,
}

#[derive(Clone)]
pub struct CameraSnapshot {
    pub position: Vec3,
    pub target: Vec3,
    pub fov: f32,
    pub time: f32,
}

impl CameraLog {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: vec![
                CameraSnapshot {
                    position: Vec3::ZERO,
                    target: Vec3::ZERO,
                    fov: 0.0,
                    time: 0.0,
                };
                capacity
            ],
            capacity,
            head: 0,
            count: 0,
        }
    }

    /// Record a camera snapshot. Only records if position or target changed.
    pub fn record(&mut self, camera: &Camera, time: f32) {
        if let Some(last) = self.latest() {
            let pos_same = (last.position - camera.position).length() < 1e-6;
            let tgt_same = (last.target - camera.target).length() < 1e-6;
            if pos_same && tgt_same {
                return;
            }
        }

        self.entries[self.head] = CameraSnapshot {
            position: camera.position,
            target: camera.target,
            fov: camera.fov,
            time,
        };
        self.head = (self.head + 1) % self.capacity;
        self.count = self.count.saturating_add(1).min(self.capacity);
    }

    /// Get the most recent snapshot.
    pub fn latest(&self) -> Option<&CameraSnapshot> {
        if self.count == 0 {
            return None;
        }
        let idx = if self.head == 0 {
            self.capacity - 1
        } else {
            self.head - 1
        };
        Some(&self.entries[idx])
    }

    /// Number of recorded snapshots.
    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Iterate snapshots from oldest to newest.
    pub fn iter(&self) -> impl Iterator<Item = &CameraSnapshot> {
        let start = if self.count < self.capacity {
            0
        } else {
            self.head
        };
        (0..self.count).map(move |i| &self.entries[(start + i) % self.capacity])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cam_at(x: f32, y: f32, z: f32) -> Camera {
        Camera::new(vec3(x, y, z), Vec3::ZERO)
    }

    #[test]
    fn empty_log_has_no_latest() {
        let log = CameraLog::new(10);
        assert!(log.latest().is_none());
        assert_eq!(log.len(), 0);
    }

    #[test]
    fn record_and_retrieve() {
        let mut log = CameraLog::new(10);
        log.record(&cam_at(1.0, 2.0, 3.0), 0.0);
        assert_eq!(log.len(), 1);
        let latest = log.latest().unwrap();
        assert!((latest.position - vec3(1.0, 2.0, 3.0)).length() < 1e-5);
    }

    #[test]
    fn skips_duplicate_positions() {
        let mut log = CameraLog::new(10);
        let cam = cam_at(1.0, 2.0, 3.0);
        log.record(&cam, 0.0);
        log.record(&cam, 1.0);
        log.record(&cam, 2.0);
        assert_eq!(log.len(), 1);
    }

    #[test]
    fn wraps_around_capacity() {
        let mut log = CameraLog::new(3);
        for i in 0..5 {
            log.record(&cam_at(i as f32, 0.0, 0.0), i as f32);
        }
        assert_eq!(log.len(), 3);
        let latest = log.latest().unwrap();
        assert!((latest.position.x - 4.0).abs() < 1e-5);
    }

    #[test]
    fn iter_oldest_to_newest() {
        let mut log = CameraLog::new(3);
        for i in 0..5 {
            log.record(&cam_at(i as f32, 0.0, 0.0), i as f32);
        }
        let positions: Vec<f32> = log.iter().map(|s| s.position.x).collect();
        assert_eq!(positions, vec![2.0, 3.0, 4.0]);
    }

    #[test]
    fn iter_when_not_full() {
        let mut log = CameraLog::new(10);
        log.record(&cam_at(1.0, 0.0, 0.0), 0.0);
        log.record(&cam_at(2.0, 0.0, 0.0), 1.0);
        let positions: Vec<f32> = log.iter().map(|s| s.position.x).collect();
        assert_eq!(positions, vec![1.0, 2.0]);
    }

    #[test]
    fn records_when_target_changes() {
        let mut log = CameraLog::new(10);
        log.record(&Camera::new(vec3(0.0, 0.0, 10.0), Vec3::ZERO), 0.0);
        log.record(
            &Camera::new(vec3(0.0, 0.0, 10.0), vec3(1.0, 0.0, 0.0)),
            1.0,
        );
        assert_eq!(log.len(), 2);
    }
}

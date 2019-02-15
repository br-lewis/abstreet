use derive_new::new;
use geom::{Distance, Duration, Speed, EPSILON_DIST};
use std::fmt;

#[derive(Debug)]
pub struct Interval {
    pub start_dist: Distance,
    pub end_dist: Distance,
    pub start_time: Duration,
    pub end_time: Duration,
    pub start_speed: Speed,
    pub end_speed: Speed,
    // Extra info: CarID, LaneID
}

impl Interval {
    pub fn new(
        start_dist: Distance,
        end_dist: Distance,
        start_time: Duration,
        end_time: Duration,
        start_speed: Speed,
        end_speed: Speed,
    ) -> Interval {
        if start_dist < Distance::ZERO {
            panic!("Interval start_dist: {}", start_dist);
        }
        if start_time < Duration::ZERO {
            panic!("Interval start_time: {}", start_time);
        }
        // TODO And the epsilons creep in...
        if start_dist > end_dist + Distance::EPSILON {
            panic!("Interval {} .. {}", start_dist, end_dist);
        }
        if start_time >= end_time {
            panic!("Interval {} .. {}", start_time, end_time);
        }
        if start_speed < Speed::ZERO {
            panic!("Interval start_speed: {}", start_speed);
        }
        if end_speed < Speed::ZERO {
            panic!("Interval end_speed: {}", end_speed);
        }
        Interval {
            start_dist,
            end_dist,
            start_time,
            end_time,
            start_speed,
            end_speed,
        }
    }

    pub fn dist(&self, t: Duration) -> Distance {
        // Linearly interpolate
        self.start_dist + self.percent(t) * (self.end_dist - self.start_dist)
    }

    pub fn speed(&self, t: Duration) -> Speed {
        // Linearly interpolate
        let result = self.start_speed + self.percent(t) * (self.end_speed - self.start_speed);
        // TODO Happens because of slight epsilonness, yay
        result.max(Speed::ZERO)
    }

    pub fn covers(&self, t: Duration) -> bool {
        t >= self.start_time - Duration::EPSILON && t <= self.end_time + Duration::EPSILON
    }

    pub fn percent(&self, t: Duration) -> f64 {
        assert!(self.covers(t));
        (t - self.start_time) / (self.end_time - self.start_time)
    }

    // TODO Also the speed at that time. Adjust all of them if they're slightly OOB.
    pub fn intersection(&self, other: &Interval) -> Option<(Duration, Distance)> {
        if !overlap(
            (self.start_time, self.end_time),
            (other.start_time, other.end_time),
        ) {
            return None;
        }
        // TODO Should bake in an epsilon check...
        if !overlap(
            (self.start_dist - EPSILON_DIST, self.end_dist + EPSILON_DIST),
            (
                other.start_dist - EPSILON_DIST,
                other.end_dist + EPSILON_DIST,
            ),
        ) {
            return None;
        }

        // Set the two distance equations equal and solve for time. Long to type out here...
        let x1 = self.start_dist.inner_meters();
        let x2 = self.end_dist.inner_meters();
        let a1 = self.start_time.inner_seconds();
        let a2 = self.end_time.inner_seconds();

        let y1 = other.start_dist.inner_meters();
        let y2 = other.end_dist.inner_meters();
        let b1 = other.start_time.inner_seconds();
        let b2 = other.end_time.inner_seconds();

        let numer = a1 * (b2 * (y1 - x2) + b1 * (x2 - y2)) + a2 * (b2 * (x1 - y1) + b1 * (y2 - x1));
        let denom = (a1 - a2) * (y1 - y2) + b2 * (x1 - x2) + b1 * (x2 - x1);
        if denom == 0.0 {
            return None;
        }
        let t = Duration::seconds(numer / denom);

        if !self.covers(t) || !other.covers(t) {
            return None;
        }

        let dist1 = self.dist(t);
        let dist2 = other.dist(t);
        if !dist1.epsilon_eq(dist2) {
            panic!(
                "{:?} and {:?} intersect at {}, but got dist {} and {}",
                self, other, t, dist1, dist2
            );
        }
        Some((t, dist1))
    }
}

impl fmt::Display for Interval {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}->{} during {}->{} ({}->{})",
            self.start_dist,
            self.end_dist,
            self.start_time,
            self.end_time,
            self.start_speed,
            self.end_speed
        )
    }
}

// TODO debug draw an interval
// TODO debug print a bunch of intervals

fn overlap<A: PartialOrd>((a_start, a_end): (A, A), (b_start, b_end): (A, A)) -> bool {
    if a_start > b_end || b_start > a_end {
        return false;
    }
    true
}

#[derive(new)]
pub struct Delta {
    pub time: Duration,
    pub dist: Distance,
}

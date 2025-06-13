pub struct StickyLimiter {
    last_raw: i32,
    last_output: i32,
    min: i32,
    max: i32,
}

impl StickyLimiter {
    pub fn new(min: i32, max: i32) -> Self {
        Self {
            last_raw: 0,
            last_output: 0,
            min,
            max,
        }
    }

    pub fn update(&mut self, raw: i32) -> i32 {
        if raw > self.max {
            // If the raw value is above the max limit
            if raw < self.last_raw {
                // If the raw value is decreasing, adjust the output
                self.last_output -= self.last_raw - raw;
            } else {
                // Otherwise, stick to the max limit
                self.last_output = self.max;
            }
        } else if raw < self.min {
            // If the raw value is below the min limit
            if raw > self.last_raw {
                // If the raw value is increasing, adjust the output
                self.last_output += raw - self.last_raw;
            } else {
                // Otherwise, stick to the min limit
                self.last_output = self.min;
            }
        } else {
            // If the raw value is within the valid range, adjust smoothly
            self.last_output += raw - self.last_raw;
        }

        // Ensure the output stays within the valid range
        self.last_output = self.last_output.clamp(self.min, self.max);

        // Update the last raw value for the next iteration
        self.last_raw = raw;

        self.last_output
    }
}
pub struct Pid {
    kp: f32,
    ki: f32,
    kd: f32,
    last_error: f32,
    cum_error: f32,
}

impl Pid {
    pub fn new(kp: f32, ki: f32, kd: f32) -> Self {
        Pid {
            kp,
            ki,
            kd,
            last_error: 0.0,
            cum_error: 0.0,
        }
    }

    pub fn update(&mut self, target_error: f32, actual_error: f32, duration: f32) -> i16 {
        let error = target_error - actual_error; // P
        self.cum_error = self.cum_error + error * duration; // I
        let rate_error = (error - self.last_error) / duration; // D
        self.last_error = error;

        (error * self.kp + self.cum_error * self.ki + rate_error * self.kd) as i16
    }
}

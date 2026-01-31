// src/human.rs
use crate::InputDevice;
use rand::Rng;
use rand_distr::{Distribution, Normal};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub struct HumanDriver {
    pub device: Arc<Mutex<InputDevice>>,
    pub cur_x: f32,
    pub cur_y: f32,
}

impl HumanDriver {
    pub fn new(device: Arc<Mutex<InputDevice>>, start_x: u16, start_y: u16) -> Self {
        Self {
            device,
            cur_x: start_x as f32,
            cur_y: start_y as f32,
        }
    }

    // 🔥 [新增] 相对移动：用于简单的防掉线或微调
    pub fn move_relative(&mut self, dx: i32, dy: i32) {
        if let Ok(mut dev) = self.device.lock() {
            dev.mouse_move(dx, dy, 0);
        }
        self.cur_x += dx as f32;
        self.cur_y += dy as f32;
    }

    pub fn key_click(&mut self, ch: char) {
        let keycode = match ch.to_ascii_lowercase() {
            // 这个范围已经包含了所有的 a-z，公式会自动计算出正确的 HID 键码
            // 'n' -> 17 (0x11), 'o' -> 18 (0x12)
            'a'..='z' => ch.to_ascii_lowercase() as u8 - b'a' + 0x04,
            '1'..='9' => ch as u8 - b'1' + 0x1E,
            '0' => 0x27,
            ' ' => 0x2C,
            _ => 0,
        };

        if keycode != 0 {
            if let Ok(mut dev) = self.device.lock() {
                dev.key_down(keycode, 0);
                // 这里的 rng_jitter 是我们之前定义的辅助函数
                thread::sleep(Duration::from_millis(40));
                dev.key_up();
            }
        }
    }

    // 🔥 [新增] 模拟鼠标滚轮
    pub fn mouse_scroll(&mut self, delta: i32) {
        if let Ok(mut dev) = self.device.lock() {
            // lib.rs 里的 mouse_move 接受 i8 类型的 wheel
            // 这里将传入的 delta (如 -120) 转为 i8
            dev.mouse_move(0, 0, delta as i8);
        }
        thread::sleep(Duration::from_millis(100));
    }

    pub fn move_to_humanly(&mut self, target_x: u16, target_y: u16, duration_sec: f32) {
        let mut rng = rand::thread_rng();
        let start = (self.cur_x, self.cur_y);
        let end = (
            target_x as f32 + rng.gen_range(-2.0..2.0),
            target_y as f32 + rng.gen_range(-2.0..2.0),
        );

        let ctrl1 = (
            start.0 + (end.0 - start.0) * 0.2 + rng.gen_range(-40.0..40.0),
            start.1 + (end.1 - start.1) * 0.2 + rng.gen_range(-40.0..40.0),
        );
        let ctrl2 = (
            start.0 + (end.0 - start.0) * 0.8 + rng.gen_range(-20.0..60.0),
            start.1 + (end.1 - start.1) * 0.8 + rng.gen_range(-20.0..60.0),
        );

        let steps = (duration_sec * 80.0) as u32;
        let interval = Duration::from_secs_f32(duration_sec / steps as f32);

        for i in 0..=steps {
            let t_linear = i as f32 / steps as f32;
            let t_eased = Self::ease_in_out_cubic(t_linear);
            let (px, py) = Self::bezier_cubic(t_eased, start, ctrl1, ctrl2, end);

            if let Ok(mut dev) = self.device.lock() {
                dev.mouse_abs(px as u16, py as u16);
            }
            thread::sleep(interval);
        }
        self.cur_x = end.0;
        self.cur_y = end.1;
    }

    pub fn click_humanly(&mut self, left: bool, right: bool) {
        let mut rng = rand::thread_rng();
        if let Ok(mut dev) = self.device.lock() {
            dev.mouse_down(left, right);
            thread::sleep(Duration::from_millis(rng.gen_range(30..75)));
            dev.mouse_up();
        }
    }

    pub fn type_humanly(&mut self, text: &str, base_wpm: f32) {
        let base_delay_ms = 60.0 / (base_wpm * 5.0) * 1000.0;
        let normal_dist = Normal::new(base_delay_ms, base_delay_ms * 0.3).unwrap();
        let mut rng = rand::thread_rng();

        for ch in text.chars() {
            self.key_click(ch);
            let delay = normal_dist.sample(&mut rng).max(10.0) as u64;
            thread::sleep(Duration::from_millis(delay));
        }
    }

    fn ease_in_out_cubic(t: f32) -> f32 {
        if t < 0.5 {
            4.0 * t * t * t
        } else {
            1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
        }
    }

    fn bezier_cubic(
        t: f32,
        p0: (f32, f32),
        p1: (f32, f32),
        p2: (f32, f32),
        p3: (f32, f32),
    ) -> (f32, f32) {
        let u = 1.0 - t;
        let x = u.powi(3) * p0.0
            + 3.0 * u.powi(2) * t * p1.0
            + 3.0 * u * t.powi(2) * p2.0
            + t.powi(3) * p3.0;
        let y = u.powi(3) * p0.1
            + 3.0 * u.powi(2) * t * p1.1
            + 3.0 * u * t.powi(2) * p2.1
            + t.powi(3) * p3.1;
        (x, y)
    }
}

fn rng_jitter(min: u64, max: u64) -> u64 {
    rand::thread_rng().gen_range(min..max)
}

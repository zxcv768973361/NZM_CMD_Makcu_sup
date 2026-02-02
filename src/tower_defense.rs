// src/tower_defense.rs
use crate::human::HumanDriver;
use crate::nav::NavEngine;
use regex::Regex;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

// ==========================================
// 1. 数据结构协议
// ==========================================

#[derive(Debug, Clone)]
pub struct TDConfig {
    pub hud_check_rect: [i32; 4],
    pub safe_zone: [i32; 4],
    pub screen_width: f32,
    pub screen_height: f32,
}

impl Default for TDConfig {
    fn default() -> Self {
        Self {
            // 聚焦波次信息的区域
            hud_check_rect: [262, 16, 389, 97],
            safe_zone: [200, 200, 1720, 880],
            screen_width: 1920.0,
            screen_height: 1080.0,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct TrapConfigItem {
    pub name: String,
    #[serde(default)]
    pub select_pos: [i32; 2],
}

#[derive(Deserialize, Debug, Clone)]
pub struct MapMeta {
    pub grid_pixel_size: f32,
    pub offset_x: f32,
    pub offset_y: f32,
    pub bottom: f32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct BuildingExport {
    pub uid: usize,
    pub name: String,
    pub grid_x: usize,
    pub grid_y: usize,
    pub width: usize,
    pub height: usize,
    #[serde(default)]
    pub wave_num: i32,
    #[serde(default)]
    pub is_late: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub struct UpgradeEvent {
    pub building_name: String,
    pub wave_num: i32,
    pub is_late: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MapTerrainExport {
    pub map_name: String,
    pub meta: MapMeta,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MapBuildingsExport {
    pub map_name: String,
    pub buildings: Vec<BuildingExport>,
    #[serde(default)]
    pub upgrades: Vec<UpgradeEvent>,
}

#[derive(Debug, Default)]
pub struct WaveStatus {
    pub current_wave: i32,
}

// ==========================================
// 2. 塔防模块实现
// ==========================================
pub struct TowerDefenseApp {
    driver: Arc<Mutex<HumanDriver>>,
    nav: Arc<NavEngine>,
    config: TDConfig,
    map_meta: Option<MapMeta>,

    // 策略数据
    strategy_buildings: Vec<BuildingExport>,
    strategy_upgrades: Vec<UpgradeEvent>,

    // 状态追踪
    placed_uids: HashSet<usize>,
    completed_upgrade_keys: HashSet<String>,
    last_confirmed_wave: i32,
    last_wave_change_time: Instant,

    trap_lookup: HashMap<String, TrapConfigItem>,
    active_loadout: Vec<String>,
    camera_offset_x: f32, // 🔥 补回缺失字段
    camera_offset_y: f32,
    move_speed: f32,
}

impl TowerDefenseApp {
    pub fn new(driver: Arc<Mutex<HumanDriver>>, nav: Arc<NavEngine>) -> Self {
        Self {
            driver,
            nav,
            config: TDConfig::default(),
            map_meta: None,
            strategy_buildings: Vec::new(),
            strategy_upgrades: Vec::new(),
            placed_uids: HashSet::new(),
            completed_upgrade_keys: HashSet::new(),
            last_confirmed_wave: 0,
            last_wave_change_time: Instant::now(),
            trap_lookup: HashMap::new(),
            active_loadout: Vec::new(),
            camera_offset_x: 0.0, // 🔥 补回初始化
            camera_offset_y: 0.0,
            move_speed: 720.0,
        }
    }

    // --- 数据加载 ---

    pub fn load_strategy(&mut self, path: &str) {
        if let Ok(c) = fs::read_to_string(path) {
            if let Ok(data) = serde_json::from_str::<MapBuildingsExport>(&c) {
                self.strategy_buildings = data.buildings;
                self.strategy_upgrades = data.upgrades;
                println!(
                    "🏗️ 策略加载成功: {}个建筑, {}个升级任务",
                    self.strategy_buildings.len(),
                    self.strategy_upgrades.len()
                );
            }
        }
    }

    // --- 波次识别逻辑 ---

    pub fn recognize_wave_status(&self) -> Option<WaveStatus> {
        let text: String = self.nav.ocr_area(self.config.hud_check_rect);
        if text.is_empty() {
            return None;
        }

        let re_wave = Regex::new(r"波次(\d+)").unwrap();

        if let Some(caps) = re_wave.captures(&text) {
            let val = caps.get(1)?.as_str().parse::<i32>().ok()?;
            Some(WaveStatus { current_wave: val })
        } else {
            None
        }
    }

    /// 验证波次跳变是否合法
    fn validate_wave_transition(&mut self, detected_wave: i32) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_wave_change_time).as_secs();

        // 1. 必须是下一波 (n -> n+1) 或初始状态
        let is_next_wave = detected_wave == self.last_confirmed_wave + 1;
        // 2. 间隔必须大于 60 秒
        let is_long_enough = elapsed >= 60 || self.last_confirmed_wave == 0;

        if is_next_wave && is_long_enough {
            println!(
                "✅ [Monitor] 确认进入新波次: {} -> {}",
                self.last_confirmed_wave, detected_wave
            );
            self.last_confirmed_wave = detected_wave;
            self.last_wave_change_time = now;
            true
        } else {
            false
        }
    }

    // --- 执行逻辑 ---

    pub fn execute_wave_phase(&mut self, wave: i32, is_late: bool) {
        let phase_name = if is_late { "后期" } else { "前期" };
        println!("🚀 开始执行第 {} 波 [{}] 布防任务...", wave, phase_name);

        let to_place: Vec<BuildingExport> = self
            .strategy_buildings
            .iter()
            .filter(|b| {
                b.wave_num == wave && b.is_late == is_late && !self.placed_uids.contains(&b.uid)
            })
            .cloned()
            .collect();

        if !to_place.is_empty() {
            self.execute_specific_placements(to_place);
        }

        let to_upgrade: Vec<UpgradeEvent> = self
            .strategy_upgrades
            .iter()
            .filter(|u| u.wave_num == wave && u.is_late == is_late)
            .filter(|u| {
                let key = format!("{}-{}-{}", u.building_name, u.wave_num, u.is_late);
                !self.completed_upgrade_keys.contains(&key)
            })
            .cloned()
            .collect();

        if !to_upgrade.is_empty() {
            self.execute_specific_upgrades(to_upgrade);
        }
    }

    fn execute_specific_placements(&mut self, tasks: Vec<BuildingExport>) {
        let mut last_key: Option<char> = None;
        let [sz_x1, sz_y1, sz_x2, sz_y2] = self.config.safe_zone;

        for b in tasks {
            if let Some((map_px, map_py)) =
                self.get_absolute_map_pixel(b.grid_x, b.grid_y, b.width, b.height)
            {
                self.ensure_target_in_safe_zone(map_px, map_py);

                let screen_x = map_px - self.camera_offset_x;
                let screen_y = map_py - self.camera_offset_y;
                let final_x = screen_x.clamp(sz_x1 as f32, sz_x2 as f32);
                let final_y = screen_y.clamp(sz_y1 as f32, sz_y2 as f32);

                let key = self.get_trap_key(&b.name);
                if let Ok(mut d) = self.driver.lock() {
                    d.move_to_humanly(final_x as u16, final_y as u16, 0.35);
                    if Some(key) != last_key {
                        d.key_click(key);
                        last_key = Some(key);
                        thread::sleep(Duration::from_millis(200));
                    }
                    d.double_click_humanly(true, false);
                }
                self.placed_uids.insert(b.uid);
                thread::sleep(Duration::from_millis(250));
            }
        }
    }

    fn execute_specific_upgrades(&mut self, tasks: Vec<UpgradeEvent>) {
        for u in tasks {
            let key = self.get_trap_key(&u.building_name);
            if let Ok(mut d) = self.driver.lock() {
                d.key_click(key);
                thread::sleep(Duration::from_millis(200));
                d.key_click('u');
            }
            let key_str = format!("{}-{}-{}", u.building_name, u.wave_num, u.is_late);
            self.completed_upgrade_keys.insert(key_str);
            thread::sleep(Duration::from_millis(400));
        }
    }

    // --- 辅助工具 ---

    fn ensure_target_in_safe_zone(&mut self, _tx: f32, ty: f32) {
        let meta = match &self.map_meta {
            Some(m) => m,
            None => return,
        };
        let [_, z_y1, _, z_y2] = self.config.safe_zone;
        let max_offset_y = (meta.bottom - self.config.screen_height).max(0.0);
        let is_bottom_zone = ty > (meta.bottom - (self.config.screen_height - z_y1 as f32));

        loop {
            let rel_y = ty - self.camera_offset_y;
            if rel_y >= z_y1 as f32 && rel_y <= z_y2 as f32 {
                break;
            }
            let target_offset = if is_bottom_zone {
                max_offset_y
            } else {
                let safe_center_y = (z_y1 + z_y2) as f32 / 2.0;
                (self.camera_offset_y + (rel_y - safe_center_y)).clamp(0.0, max_offset_y)
            };
            let dist = target_offset - self.camera_offset_y;
            if dist.abs() < 5.0 {
                break;
            }

            if let Ok(mut human) = self.driver.lock() {
                let key = if dist > 0.0 { 's' } else { 'w' };
                human.key_hold(key, (dist.abs() / self.move_speed * 1000.0) as u64);
                self.camera_offset_y = target_offset;
            }
            thread::sleep(Duration::from_millis(400));
            if is_bottom_zone {
                break;
            }
        }
    }

    pub fn load_map_terrain(&mut self, path: &str) {
        if let Ok(c) = fs::read_to_string(path) {
            if let Ok(data) = serde_json::from_str::<MapTerrainExport>(&c) {
                self.map_meta = Some(data.meta);
            }
        }
    }

    pub fn load_trap_config(&mut self, json_path: &str) {
        if let Ok(c) = fs::read_to_string(json_path) {
            if let Ok(items) = serde_json::from_str::<Vec<TrapConfigItem>>(&c) {
                for item in items {
                    self.trap_lookup.insert(item.name.clone(), item);
                }
            }
        }
    }

    pub fn setup_view(&mut self) {
        println!("🔭 对齐左上角边界...");
        if let Ok(mut human) = self.driver.lock() {
            human.key_click('o');
            thread::sleep(Duration::from_secs(2));
            for _ in 1..=7 {
                for _ in 0..12 {
                    human.mouse_scroll(-120);
                    thread::sleep(Duration::from_millis(30));
                }
                thread::sleep(Duration::from_millis(300));
            }
            for _ in 1..=4 {
                human.key_hold('w', 500);
                thread::sleep(Duration::from_millis(50));
                human.key_hold('a', 500);
                thread::sleep(Duration::from_millis(50));
            }
            human.key_hold('w', 800);
            human.key_hold('a', 800);
        }
        self.camera_offset_x = 0.0; // 🔥 已正确识别字段
        self.camera_offset_y = 0.0;
    }

    pub fn execute_prep_logic(&self, loadout: &[&str]) {
        println!("🔧 执行赛前准备...");
        if let Ok(mut human) = self.driver.lock() {
            human.key_click('n');
            thread::sleep(Duration::from_millis(1000));
            human.move_to_humanly(212, 294, 0.5);
            human.click_humanly(true, false, 0);
        }
        self.select_loadout(loadout);
        if let Ok(mut human) = self.driver.lock() {
            human.key_click('n');
            thread::sleep(Duration::from_millis(500));
        }
    }

    pub fn select_loadout(&self, tower_names: &[&str]) {
        for name in tower_names.iter().take(4) {
            if let Some(config) = self.trap_lookup.get(*name) {
                let [x, y] = config.select_pos;
                if let Ok(mut d) = self.driver.lock() {
                    d.move_to_humanly(x as u16, y as u16, 0.5);
                    d.click_humanly(true, false, 0);
                }
                thread::sleep(Duration::from_millis(400));
            }
        }
    }

    fn get_absolute_map_pixel(
        &self,
        gx: usize,
        gy: usize,
        w: usize,
        h: usize,
    ) -> Option<(f32, f32)> {
        let meta = self.map_meta.as_ref()?;
        let sx = meta.offset_x + ((gx as f32 + w as f32 / 2.0) * meta.grid_pixel_size);
        let sy = meta.offset_y + ((gy as f32 + h as f32 / 2.0) * meta.grid_pixel_size);
        Some((sx, sy))
    }

    fn get_trap_key(&self, name: &str) -> char {
        let index = self
            .active_loadout
            .iter()
            .position(|t| t == name)
            .unwrap_or(0);
        match index {
            0 => '4',
            1 => '5',
            2 => '6',
            3 => '7',
            _ => '1',
        }
    }

    // --- 全自动入口 ---

    pub fn run(&mut self, terrain_p: &str, strategy_p: &str, trap_p: &str, loadout: &[&str]) {
        self.active_loadout = loadout.iter().map(|&s| s.to_string()).collect();
        self.load_map_terrain(terrain_p);
        self.load_strategy(strategy_p);
        self.load_trap_config(trap_p);

        if let Ok(mut human) = self.driver.lock() {
            println!("👆 点击游戏入口/开始按钮...");
            human.move_to_humanly(1700, 950, 0.5);
            human.click_humanly(true, false, 0);

            human.move_to_humanly(1110, 670, 0.5);
            human.click_humanly(true, false, 0);
        }

        println!("⏳ 等待进入战斗关卡（监控波次中）...");
        loop {
            if let Some(status) = self.recognize_wave_status() {
                if status.current_wave > 0 {
                    println!("🎮 检测到战斗已开始！当前确认波次: {}", status.current_wave);
                    self.last_wave_change_time = Instant::now();
                    break;
                }
            }
            thread::sleep(Duration::from_millis(1000));
        }

        self.execute_prep_logic(loadout);
        self.setup_view();

        println!("🤖 进入自动化监控主循环...");

        loop {
            if let Some(status) = self.recognize_wave_status() {
                if self.validate_wave_transition(status.current_wave) {
                    let current_wave = status.current_wave;
                    self.execute_wave_phase(current_wave, false);

                    println!(
                        "🔔 第 {} 波前期布防完成，按下 'G' 启动战斗阶段",
                        current_wave
                    );
                    if let Ok(mut d) = self.driver.lock() {
                        d.key_click('g');
                    }
                    thread::sleep(Duration::from_secs(1));

                    self.execute_wave_phase(current_wave, true);
                }
            }
            thread::sleep(Duration::from_millis(1500));
        }
    }
}

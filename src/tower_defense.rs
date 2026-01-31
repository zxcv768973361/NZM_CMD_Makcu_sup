// src/tower_defense.rs
use crate::human::HumanDriver;
use crate::nav::NavEngine;
use serde::Deserialize;
use std::fs;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

// ==========================================
// 1. 配置与协议定义
// ==========================================
#[derive(Debug, Clone)]
pub struct TDConfig {
    pub hud_check_rect: [i32; 4], 
}

impl Default for TDConfig {
    fn default() -> Self {
        Self {
            // 🎯 OCR 区域：897, 110, 1030, 145  
            hud_check_rect: [845, 88, 1098, 175],
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct MapMeta {
    pub grid_pixel_size: f32,
    pub offset_x: f32,
    pub offset_y: f32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct BuildingExport {
    pub name: String,
    pub grid_x: usize,
    pub grid_y: usize,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MapTerrainExport {
    pub meta: MapMeta,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MapBuildingsExport {
    pub buildings: Vec<BuildingExport>,
}

// ==========================================
// 2. 塔防模块实现
// ==========================================
pub struct TowerDefenseApp {
    driver: Arc<Mutex<HumanDriver>>,
    nav: Arc<NavEngine>,
    config: TDConfig,
    map_meta: Option<MapMeta>,
    strategy: Vec<BuildingExport>,
}

impl TowerDefenseApp {
    pub fn new(driver: Arc<Mutex<HumanDriver>>, nav: Arc<NavEngine>) -> Self {
        Self {
            driver,
            nav,
            config: TDConfig::default(),
            map_meta: None,
            strategy: Vec::new(),
        }
    }

    pub fn init_game_start(&self, timeout_secs: u64) -> bool {
        println!("========================================");
        println!("🎮 [TD] 准备进入游戏...");

        if let Ok(mut d) = self.driver.lock() {
            d.move_to_humanly(1700, 950, 0.5);
            d.click_humanly(true, false);
            thread::sleep(Duration::from_millis(800));
            d.move_to_humanly(1103, 671, 0.5);
            d.click_humanly(true, false);
        }

        let start_time = Instant::now();
        while start_time.elapsed().as_secs() < timeout_secs {
            let ocr_text = self.nav.ocr_area(self.config.hud_check_rect);

            // 🔥 [调试打印] 让你看到程序到底识别出了什么
            println!("      [DEBUG] OCR 区域文字: [{}]", ocr_text);

            if ocr_text.contains("怪物") || ocr_text.contains("将来") || ocr_text.contains("袭") {
                println!("✅ [TD] 识别成功！");
                thread::sleep(Duration::from_secs(2));
                return true;
            }

            if let Ok(mut d) = self.driver.lock() {
                d.move_relative(1, 1);
                thread::sleep(Duration::from_millis(100));
                d.move_relative(-1, -1);
            }
            thread::sleep(Duration::from_secs(1));
        }
        println!("❌ [TD] 游戏加载超时！");
        false
    }

    pub fn load_map(&mut self, terrain_path: &str, strategy_path: &str) {
        if let Ok(c) = fs::read_to_string(terrain_path) {
            if let Ok(data) = serde_json::from_str::<MapTerrainExport>(&c) { self.map_meta = Some(data.meta); }
        }
        if let Ok(c) = fs::read_to_string(strategy_path) {
            if let Ok(data) = serde_json::from_str::<MapBuildingsExport>(&c) { self.strategy = data.buildings; }
        }
    }

    fn grid_to_screen(&self, gx: usize, gy: usize) -> Option<(i32, i32)> {
        let meta = self.map_meta.as_ref()?;
        let sx = meta.offset_x + (gx as f32 * meta.grid_pixel_size) + (meta.grid_pixel_size / 2.0);
        let sy = meta.offset_y + (gy as f32 * meta.grid_pixel_size) + (meta.grid_pixel_size / 2.0);
        Some((sx as i32, sy as i32))
    }

    fn get_trap_key(&self, name: &str) -> char {
        match name { "自修复磁暴塔" => '1', "破坏者" => '2', "减速陷阱" => '3', "防空导弹" => '4', _ => '1' }
    }

    pub fn setup_view(&self) {
        if let Ok(mut d) = self.driver.lock() {
            d.key_click('o');
            thread::sleep(Duration::from_secs(2));
            for _ in 0..8 { d.mouse_scroll(-120); thread::sleep(Duration::from_millis(50)); }
        }
    }

    pub fn execute_all_placements(&self) {
        for b in &self.strategy {
            if let Some((sx, sy)) = self.grid_to_screen(b.grid_x, b.grid_y) {
                let key = self.get_trap_key(&b.name);
                if let Ok(mut d) = self.driver.lock() {
                    d.move_to_humanly(sx as u16, sy as u16, 0.4);
                    d.key_click(key);
                    thread::sleep(Duration::from_millis(100));
                    d.click_humanly(true, false);
                }
                thread::sleep(Duration::from_millis(200));
            }
        }
    }

    pub fn run(&mut self, terrain_json: &str, strategy_json: &str) {
        if !self.init_game_start(120) { return; }
        self.load_map(terrain_json, strategy_json);
        self.setup_view();
        self.execute_all_placements();
    }
}